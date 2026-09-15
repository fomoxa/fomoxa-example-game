//go:build !kaiju

package main

import "fmt"

func main() {
	fmt.Println("this file is the Kaiju client; build it inside a Kaiju engine checkout with tools/kaiju.sh build")
}
