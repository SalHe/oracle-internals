use std::fmt::Display;

use zerocopy::{FromBytes, Immutable, KnownLayout};

#[derive(KnownLayout, Immutable, FromBytes, Debug, Clone, Copy)]
#[repr(transparent)]
pub struct OracleTimestamp(pub u32);

impl OracleTimestamp {
    fn _real_timestamp(&self) -> u32 {
        if self.0 == 0 {
            // 感觉时间的计算可能得再看看
            // 根据dump文件头 Terminal recovery 时间是：01/01/1988 00:00:00
            // 结合汇编来看，Terminal recovery=0，与其他非0时间戳的偏移不一样？
            return 567964800;
        }
        self.0 + 545961600
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DateTime {
    pub year: u16,   // 年份 (从1988开始计算)
    pub month: u16,  // 月份 (1-12)
    pub day: u16,    // 日期 (1-31)
    pub hour: u16,   // 小时 (0-23)
    pub minute: u16, // 分钟 (0-59)
    pub second: u16, // 秒数 (0-59)
}

impl From<OracleTimestamp> for DateTime {
    fn from(value: OracleTimestamp) -> Self {
        Self::from(&value)
    }
}

impl From<&OracleTimestamp> for DateTime {
    fn from(value: &OracleTimestamp) -> Self {
        let ts = value.0;
        let pure_minutes = ts / 0x3C;
        let seconds_of_minutes = 60 * (ts / 0x3C);
        let pure_hours = ts / 0x3C / 0x3C;

        let magic = 0x8421085u64; // ???? what's this ????
        let v6 = (((pure_hours as u64 / 24 * magic) >> 32)
            + (((pure_hours as u64 / 24 - ((pure_hours as u64 / 24 * magic) >> 32)) as u32) >> 1)
                as u64) as u32
            >> 4;

        let _ = 0xAAAAAAAB as u64 * v6 as u64; // ???
        Self {
            year: (v6 / 0xC + 1988) as _,
            month: (v6 % 0xC + 1) as _,
            day: (pure_hours / 24 - 31 * v6 + 1) as _,
            hour: (pure_hours % 24) as _,
            minute: (pure_minutes - 60 * pure_hours) as _,
            second: (ts - seconds_of_minutes) as _,
        }
    }
}

impl Display for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:02}/{:02}/{:04} {:02}:{:02}:{:02}",
            self.month, self.day, self.year, self.hour, self.minute, self.second,
        )
    }
}
