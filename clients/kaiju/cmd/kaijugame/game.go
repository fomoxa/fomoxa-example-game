//go:build kaiju

package main

import (
	"fmt"
	"log/slog"
	"os"
	"path/filepath"
	"reflect"
	"slices"
	"strings"
	"time"

	"kaijuengine.com/bootstrap"
	"kaijuengine.com/engine"
	"kaijuengine.com/engine/assets"
	"kaijuengine.com/engine/ui"
	"kaijuengine.com/fomoxa_example/network"
	"kaijuengine.com/fomoxa_example/network/models"
	"kaijuengine.com/matrix"
	"kaijuengine.com/platform/hid"
	"kaijuengine.com/registry/shader_data_registry"
	"kaijuengine.com/rendering"
	"kaijuengine.com/rendering/textures"
)

const (
	gameContentPath    = `game_content`
	defaultAddress     = "127.0.0.1:9321"
	defaultName        = "kaiju"
	defaultSensitivity = float32(0.0035)
	eyeHeight          = float32(1.5)
	bodyHeight         = float32(1.2)
	bodyWidth          = float32(0.6)
	headSize           = float32(0.45)
)

type avatar struct {
	body       *engine.Entity
	head       *engine.Entity
	bodyShader *shader_data_registry.ShaderDataStandard
	headShader *shader_data_registry.ShaderDataStandard
	inUse      bool
}

type Game struct {
	host        *engine.Host
	session     *network.Session
	address     string
	displayName string
	sensitivity float32
	floor       *engine.Entity
	ui          *ui.Manager
	label       *ui.Label
	material    *rendering.Material
	texture     *rendering.Texture
	cube        *rendering.Mesh
	avatars     map[uint32]*avatar
	pool        []*avatar
	yaw         float32
	pitch       float32
	lastMouse   matrix.Vec2
	looking     bool
}

func (Game) PluginRegistry() []reflect.Type { return []reflect.Type{} }

func (Game) ContentDatabase() (assets.Database, error) {
	if _, err := os.Stat(gameContentPath); err != nil {
		if !os.IsNotExist(err) {
			return nil, err
		}
		if err := copyEditorContent(); err != nil {
			return nil, err
		}
	}
	return assets.NewFileDatabase(gameContentPath)
}

func (g *Game) Launch(host *engine.Host) {
	g.host = host
	g.session = network.NewSession()
	g.avatars = make(map[uint32]*avatar)

	material, err := host.MaterialCache().Material(assets.MaterialDefinitionBasic)
	if err != nil {
		panic("fomoxa example: the asset database has no basic material")
	}
	texture, err := host.TextureCache().Texture(assets.TextureSquare, textures.TextureFilterLinear)
	if err != nil {
		panic("fomoxa example: the asset database has no square texture")
	}
	g.material = material
	g.texture = texture
	g.cube = rendering.NewMeshCube(host.MeshCache())

	g.buildFloor()
	g.buildHud()
	g.connect()

	updateID := host.Updater.AddUpdate(g.update)
	uiUpdateID := host.UIUpdater.AddUpdate(g.updateHud)
	g.floor.OnDestroy.Add(func() {
		host.Updater.RemoveUpdate(&updateID)
		host.UIUpdater.RemoveUpdate(&uiUpdateID)
		g.session.Close()
	})
}

func (g *Game) buildFloor() {
	shaderData := shader_data_registry.Create("basic").(*shader_data_registry.ShaderDataStandard)
	shaderData.Color = matrix.NewColor(0.16, 0.18, 0.22, 1)
	g.floor = engine.NewEntity(g.host.WorkGroup())
	g.floor.Transform.SetScale(matrix.NewVec3(20, 1, 20))
	g.host.Drawings.AddDrawing(rendering.Drawing{
		Material:   g.material.CreateInstance([]*rendering.Texture{g.texture}),
		Mesh:       rendering.NewMeshPlane(g.host.MeshCache()),
		ShaderData: shaderData,
		Transform:  &g.floor.Transform,
		ViewCuller: &g.host.Cameras.Primary,
	})
}

func (g *Game) buildHud() {
	g.ui = &ui.Manager{}
	g.ui.Init(g.host)
	g.label = g.ui.Add().ToLabel()
	g.label.Init("fomoxa: connecting")
	g.label.SetColor(matrix.ColorWhite())
	g.label.SetBGColor(matrix.ColorTransparent())
	panel := g.ui.Add().ToPanel()
	panel.Init(g.texture, ui.ElementTypePanel)
	panel.SetColor(matrix.ColorTransparent())
	panel.AddChild(g.label.Base())
}

func (g *Game) connect() {
	g.address = envOr("FOMOXA_EXAMPLE_ADDR", defaultAddress)
	g.displayName = envOr("FOMOXA_EXAMPLE_NAME", defaultName)
	g.sensitivity = defaultSensitivity
	if err := g.session.Open(g.address, g.displayName); err != nil {
		slog.Error("fomoxa example: cannot connect", "error", err)
	}
}

func (g *Game) update(deltaTime float64) {
	if err := g.session.Poll(time.Now()); err != nil {
		slog.Error("fomoxa example: session lost", "error", err)
	}

	keyboard := g.host.Window.Keyboard
	if keyboard.KeyHeld(hid.KeyboardKeyR) && !g.session.Joined() {
		g.connect()
	}

	g.readLook()
	forward, strafe, jump := readMove(keyboard)
	moveX, moveZ := network.WorldMove(strafe, forward, g.yaw)
	if err := g.session.SendInput(moveX, moveZ, jump, g.yaw, g.pitch); err != nil {
		slog.Error("fomoxa example: cannot send input", "error", err)
	}

	g.syncAvatars()
	g.placeCamera()
}

func (g *Game) readLook() {
	mouse := g.host.Window.Mouse
	position := mouse.Position()
	if !mouse.Held(hid.MouseButtonLeft) {
		g.looking = false
		return
	}
	if !g.looking {
		g.looking = true
		g.lastMouse = position
		return
	}
	delta := matrix.Vec2{position.X() - g.lastMouse.X(), position.Y() - g.lastMouse.Y()}
	g.lastMouse = position
	g.yaw = network.WrapAngle(g.yaw - delta.X()*g.sensitivity)
	g.pitch = network.ClampPitch(g.pitch - delta.Y()*g.sensitivity)
}

func readMove(keyboard hid.Keyboard) (float32, float32, bool) {
	var forward, strafe float32
	if keyboard.KeyHeld(hid.KeyboardKeyW) || keyboard.KeyHeld(hid.KeyboardKeyUp) {
		forward++
	}
	if keyboard.KeyHeld(hid.KeyboardKeyS) || keyboard.KeyHeld(hid.KeyboardKeyDown) {
		forward--
	}
	if keyboard.KeyHeld(hid.KeyboardKeyD) || keyboard.KeyHeld(hid.KeyboardKeyRight) {
		strafe++
	}
	if keyboard.KeyHeld(hid.KeyboardKeyA) || keyboard.KeyHeld(hid.KeyboardKeyLeft) {
		strafe--
	}
	return forward, strafe, keyboard.KeyHeld(hid.KeyboardKeySpace)
}

func (g *Game) syncAvatars() {
	local := g.session.LocalID()
	live := make(map[uint32]struct{}, g.session.PlayerCount())
	for _, id := range g.session.PlayerIDs() {
		state, ok := g.session.Player(id)
		if !ok {
			continue
		}
		live[id] = struct{}{}
		if id == local {
			continue
		}
		g.placeAvatar(state)
	}
	for id, entry := range g.avatars {
		if _, ok := live[id]; ok && id != local {
			continue
		}
		entry.body.Deactivate()
		entry.head.Deactivate()
		entry.inUse = false
		g.pool = append(g.pool, entry)
		delete(g.avatars, id)
	}
}

func (g *Game) placeAvatar(state models.PlayerState) {
	entry, ok := g.avatars[state.PlayerID]
	if !ok {
		entry = g.takeAvatar()
		g.avatars[state.PlayerID] = entry
	}

	color := colorOf(state.Color)
	entry.bodyShader.Color = color
	entry.headShader.Color = matrix.NewColor(color.R()*0.6, color.G()*0.6, color.B()*0.6, 1)

	body := matrix.NewVec3(state.PositionX, state.PositionY+bodyHeight*0.5, state.PositionZ)
	entry.body.Transform.SetPosition(body)
	entry.body.Transform.SetScale(matrix.NewVec3(bodyWidth, bodyHeight, bodyWidth))
	entry.head.Transform.SetPosition(matrix.NewVec3(body.X(), state.PositionY+bodyHeight+headSize*0.5, body.Z()))
	entry.head.Transform.SetScale(matrix.NewVec3(headSize, headSize, headSize))
	entry.head.Transform.SetRotation(matrix.NewVec3(0, radiansToDegrees(state.LookYaw), 0))
}

func (g *Game) takeAvatar() *avatar {
	if len(g.pool) > 0 {
		entry := g.pool[len(g.pool)-1]
		g.pool = g.pool[:len(g.pool)-1]
		entry.body.Activate()
		entry.head.Activate()
		entry.inUse = true
		return entry
	}

	entry := &avatar{
		body:       engine.NewEntity(g.host.WorkGroup()),
		head:       engine.NewEntity(g.host.WorkGroup()),
		bodyShader: shader_data_registry.Create("basic").(*shader_data_registry.ShaderDataStandard),
		headShader: shader_data_registry.Create("basic").(*shader_data_registry.ShaderDataStandard),
		inUse:      true,
	}
	g.host.Drawings.AddDrawing(rendering.Drawing{
		Material:   g.material.CreateInstance([]*rendering.Texture{g.texture}),
		Mesh:       g.cube,
		ShaderData: entry.bodyShader,
		Transform:  &entry.body.Transform,
		ViewCuller: &g.host.Cameras.Primary,
	})
	g.host.Drawings.AddDrawing(rendering.Drawing{
		Material:   g.material.CreateInstance([]*rendering.Texture{g.texture}),
		Mesh:       g.cube,
		ShaderData: entry.headShader,
		Transform:  &entry.head.Transform,
		ViewCuller: &g.host.Cameras.Primary,
	})
	return entry
}

func (g *Game) placeCamera() {
	camera := g.host.Cameras.Primary.Camera
	if camera == nil {
		return
	}
	state, ok := g.session.LocalPlayer()
	if !ok {
		camera.SetPositionAndLookAt(matrix.NewVec3(0, 12, 18), matrix.NewVec3(0, 0, 0))
		return
	}
	eye := matrix.NewVec3(state.PositionX, state.PositionY+eyeHeight, state.PositionZ)
	x, y, z := network.LookDirection(g.yaw, g.pitch)
	camera.SetPositionAndLookAt(eye, matrix.NewVec3(eye.X()+x, eye.Y()+y, eye.Z()+z))
}

func (g *Game) updateHud(deltaTime float64) {
	if failure := g.session.Failure(); failure != "" {
		g.label.SetText(fmt.Sprintf("fomoxa: %s - press R to retry", failure))
		return
	}
	if !g.session.Joined() {
		g.label.SetText(fmt.Sprintf("fomoxa: %s %s", g.session.Status(), g.address))
		return
	}
	g.label.SetText(fmt.Sprintf(
		"fomoxa kaiju #%d · tick %d · %d players · %.0f Hz · hold left mouse to look, WASD to move, Space to jump",
		g.session.LocalID(),
		g.session.Tick(),
		g.session.PlayerCount(),
		1/deltaTime,
	))
}

func colorOf(packed uint32) matrix.Color {
	return matrix.NewColor(
		float32((packed>>24)&0xFF)/255,
		float32((packed>>16)&0xFF)/255,
		float32((packed>>8)&0xFF)/255,
		float32(packed&0xFF)/255,
	)
}

func radiansToDegrees(radians float32) float32 {
	return radians * 180 / 3.14159265358979323846
}

func envOr(name, fallback string) string {
	if value := os.Getenv(name); value != "" {
		return value
	}
	return fallback
}

func getGame() bootstrap.GameInterface { return &Game{} }

func editorContentPath() (string, error) {
	candidates := []string{
		filepath.Join("editor", "editor_embedded_content", "editor_content"),
		filepath.Join("src", "editor", "editor_embedded_content", "editor_content"),
	}
	for _, candidate := range candidates {
		if info, err := os.Stat(candidate); err == nil && info.IsDir() {
			return candidate, nil
		}
	}
	return "", fmt.Errorf("fomoxa example: no editor content in %v, run from the Kaiju checkout", candidates)
}

func copyEditorContent() error {
	rawContentPath, err := editorContentPath()
	if err != nil {
		return err
	}
	slog.Info("fomoxa example: copying stock content to the project database", "from", rawContentPath)
	if err := os.MkdirAll(gameContentPath, os.ModePerm); err != nil {
		return err
	}
	top, err := os.ReadDir(rawContentPath)
	if err != nil {
		return err
	}
	files := []string{}
	var readSubDir func(path string) error
	readSubDir = func(path string) error {
		if strings.HasSuffix(path, "renderer/src") {
			return nil
		}
		entries, err := os.ReadDir(path)
		if err != nil {
			return err
		}
		for i := range entries {
			subPath := filepath.ToSlash(filepath.Join(path, entries[i].Name()))
			if entries[i].IsDir() {
				if err := readSubDir(subPath); err != nil {
					return err
				}
				continue
			}
			files = append(files, subPath)
		}
		return nil
	}
	skip := []string{"editor"}
	for i := range top {
		if !top[i].IsDir() || slices.Contains(skip, top[i].Name()) {
			continue
		}
		if err := readSubDir(filepath.ToSlash(filepath.Join(rawContentPath, top[i].Name()))); err != nil {
			return err
		}
	}
	for i := range files {
		data, err := os.ReadFile(files[i])
		if err != nil {
			return err
		}
		if err := os.WriteFile(filepath.Join(gameContentPath, filepath.Base(files[i])), data, os.ModePerm); err != nil {
			return err
		}
	}
	return nil
}
