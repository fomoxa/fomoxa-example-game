use std::time::{SystemTime, UNIX_EPOCH};

const MILLIS_PER_DAY: u64 = 86_400_000;

pub fn timestamp() -> String {
    let since_epoch = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    format_utc_time_of_day(since_epoch.as_millis() as u64)
}

pub fn format_utc_time_of_day(unix_millis: u64) -> String {
    let millis_of_day = unix_millis % MILLIS_PER_DAY;
    format!(
        "{:02}:{:02}:{:02}.{:03}Z",
        millis_of_day / 3_600_000,
        millis_of_day / 60_000 % 60,
        millis_of_day / 1_000 % 60,
        millis_of_day % 1_000
    )
}

#[macro_export]
macro_rules! log_info {
    ($($argument:tt)*) => {
        println!("[{}] INFO  {}", $crate::log::timestamp(), format_args!($($argument)*))
    };
}

#[macro_export]
macro_rules! log_warn {
    ($($argument:tt)*) => {
        eprintln!("[{}] WARN  {}", $crate::log::timestamp(), format_args!($($argument)*))
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_the_utc_time_of_day_with_milliseconds() {
        assert_eq!(format_utc_time_of_day(1_789_462_323_045), "08:52:03.045Z");
    }

    #[test]
    fn midnight_is_all_zeroes() {
        assert_eq!(format_utc_time_of_day(MILLIS_PER_DAY * 20_000), "00:00:00.000Z");
    }
}
