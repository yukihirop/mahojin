//! 暦。特別な日や時刻に唱えると、魔法陣の色が変わる。
//!
//! 形はいつもどおりハッシュから決まる。変わるのは色（と、13 日の金曜日だけは回る向き）で、
//! 同じ呪文からは同じ形の魔法陣が出るという決まりは崩さない。

use crate::locale::Locale;

/// 暦の兆し。上にあるものほど優先する（元日が満月の夜でも元日になる）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Omen {
    /// 1 月 1 日。紅白と金
    NewYear,
    /// 7 月 7 日の夜。夜空の青と星の金
    Tanabata,
    /// 10 月 31 日。かぼちゃの橙と紫
    Halloween,
    /// 13 日の金曜日。色が抜け、逆さに回る
    Friday13,
    /// 丑三つ時（午前 2 時〜2 時半）。青白い鬼火
    Ushimitsu,
    /// 満月の夜。月の光の銀
    FullMoon,
}

/// 魔法陣の色。`(色相, 彩度, 明度)`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    pub main: (f32, f32, f32),
    pub sub: (f32, f32, f32),
    pub particle: (f32, f32, f32),
}

impl Omen {
    pub const ALL: [Omen; 6] = [
        Omen::NewYear,
        Omen::Tanabata,
        Omen::Halloween,
        Omen::Friday13,
        Omen::Ushimitsu,
        Omen::FullMoon,
    ];

    pub fn name(self, l: Locale) -> &'static str {
        match self {
            Omen::NewYear => l.pick("元日", "New Year's Day"),
            Omen::Tanabata => l.pick("七夕の夜", "Tanabata night"),
            Omen::Halloween => l.pick("ハロウィン", "Halloween"),
            Omen::Friday13 => l.pick("13 日の金曜日", "Friday the 13th"),
            Omen::Ushimitsu => l.pick("丑三つ時", "the witching hour"),
            Omen::FullMoon => l.pick("満月の夜", "full moon night"),
        }
    }

    /// 「〜に唱えたら」の「〜に」。投稿文に使う
    pub fn when(self, l: Locale) -> String {
        match (l, self) {
            (Locale::Ja, _) => format!("{}に", self.name(l)),
            (Locale::En, Omen::Ushimitsu) => "at the witching hour".into(),
            (Locale::En, Omen::FullMoon) => "on a full moon night".into(),
            (Locale::En, _) => format!("on {}", self.name(l)),
        }
    }

    /// 図鑑に書く名前
    pub fn key(self) -> &'static str {
        match self {
            Omen::NewYear => "new-year",
            Omen::Tanabata => "tanabata",
            Omen::Halloween => "halloween",
            Omen::Friday13 => "friday-13",
            Omen::Ushimitsu => "witching-hour",
            Omen::FullMoon => "full-moon",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|o| o.key() == key)
    }

    pub fn palette(self) -> Palette {
        let p = |main, sub, particle| Palette {
            main,
            sub,
            particle,
        };
        match self {
            Omen::NewYear => p((45.0, 90.0, 62.0), (355.0, 85.0, 60.0), (48.0, 100.0, 82.0)),
            Omen::Tanabata => p((215.0, 85.0, 68.0), (50.0, 95.0, 70.0), (50.0, 100.0, 85.0)),
            Omen::Halloween => p((28.0, 95.0, 58.0), (275.0, 70.0, 62.0), (275.0, 90.0, 80.0)),
            Omen::Friday13 => p((0.0, 0.0, 80.0), (0.0, 0.0, 55.0), (0.0, 0.0, 88.0)),
            Omen::Ushimitsu => p(
                (185.0, 60.0, 72.0),
                (200.0, 45.0, 52.0),
                (180.0, 80.0, 86.0),
            ),
            Omen::FullMoon => p((210.0, 25.0, 86.0), (45.0, 60.0, 72.0), (55.0, 70.0, 90.0)),
        }
    }
}

/// 暦を見るための、手元の時計の日時。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Moment {
    pub year: i32,
    pub month: u32,
    pub day: u32,
    pub hour: u32,
    pub minute: u32,
}

impl Moment {
    /// 1970-01-01 からの日数（時刻を含む）
    fn days(self) -> f64 {
        days_from_civil(self.year, self.month, self.day) as f64
            + (self.hour as f64 + self.minute as f64 / 60.0) / 24.0
    }

    /// 0 が日曜日
    fn weekday(self) -> u32 {
        // 1970-01-01 は木曜日
        (days_from_civil(self.year, self.month, self.day) + 4).rem_euclid(7) as u32
    }

    /// `2026-10-31 22:00` の形
    pub fn parse(s: &str) -> Option<Self> {
        let (date, time) = s
            .trim()
            .split_once([' ', 'T'])
            .unwrap_or((s.trim(), "00:00"));
        let mut d = date.split('-').map(str::parse::<u32>);
        let mut t = time.split(':').map(str::parse::<u32>);
        let m = Moment {
            year: d.next()?.ok()? as i32,
            month: d.next()?.ok()?,
            day: d.next()?.ok()?,
            hour: t.next()?.ok()?,
            minute: t.next().unwrap_or(Ok(0)).ok()?,
        };
        let valid = (1..=12).contains(&m.month)
            && (1..=31).contains(&m.day)
            && m.hour < 24
            && m.minute < 60;
        valid.then_some(m)
    }

    /// 手元の時計（タイムゾーン込み）。`MAHO_NOW` があればその日時にする（確かめる用）。
    pub fn now(env: impl Fn(&str) -> Option<String>) -> Option<Self> {
        if let Some(s) = env("MAHO_NOW") {
            return Self::parse(&s);
        }
        local_now()
    }
}

#[cfg(unix)]
fn local_now() -> Option<Moment> {
    // SAFETY: localtime_r は渡したバッファにだけ書く。tm はすべて整数なので 0 埋めで初期化できる
    unsafe {
        let t = libc::time(std::ptr::null_mut());
        let mut tm: libc::tm = std::mem::zeroed();
        if libc::localtime_r(&t, &mut tm).is_null() {
            return None;
        }
        Some(Moment {
            year: tm.tm_year + 1900,
            month: (tm.tm_mon + 1) as u32,
            day: tm.tm_mday as u32,
            hour: tm.tm_hour as u32,
            minute: tm.tm_min as u32,
        })
    }
}

#[cfg(not(unix))]
fn local_now() -> Option<Moment> {
    None
}

/// 朔望月（新月から次の新月まで）の平均の長さ
const SYNODIC_MONTH: f64 = 29.530_588_853;
/// 基準にする新月。2000-01-06 18:14 UTC
const NEW_MOON: f64 = 10_962.0 + (18.0 + 14.0 / 60.0) / 24.0;

/// 月齢（新月からの日数）。平均の周期で数えるので、半日ほどずれることがある
fn moon_age(m: Moment) -> f64 {
    (m.days() - NEW_MOON).rem_euclid(SYNODIC_MONTH)
}

/// その日時の兆し。何でもない日なら `None`。
pub fn at(m: Moment) -> Option<Omen> {
    let night = m.hour >= 18 || m.hour < 6;
    let full = (SYNODIC_MONTH / 2.0 - 1.0..=SYNODIC_MONTH / 2.0 + 1.0).contains(&moon_age(m));
    match (m.month, m.day) {
        (1, 1) => Some(Omen::NewYear),
        (7, 7) if m.hour >= 18 => Some(Omen::Tanabata),
        (10, 31) => Some(Omen::Halloween),
        (_, 13) if m.weekday() == 5 => Some(Omen::Friday13),
        _ if m.hour == 2 && m.minute < 30 => Some(Omen::Ushimitsu),
        _ if night && full => Some(Omen::FullMoon),
        _ => None,
    }
}

/// 1970-01-01 からの日数（Howard Hinnant の days_from_civil）
fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y } as i64;
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at_str(s: &str) -> Option<Omen> {
        at(Moment::parse(s).unwrap())
    }

    #[test]
    fn calendar_math() {
        assert_eq!(days_from_civil(1970, 1, 1), 0);
        assert_eq!(days_from_civil(2000, 1, 6), 10_962);
        // 2026-02-13 と 2026-03-13 は金曜日
        assert_eq!(Moment::parse("2026-02-13 12:00").unwrap().weekday(), 5);
        assert_eq!(Moment::parse("2026-03-13").unwrap().weekday(), 5);
        assert_eq!(Moment::parse("2026-09-24 14:00").unwrap().weekday(), 4);
        assert!(Moment::parse("2026-13-01").is_none());
        assert!(Moment::parse("tomorrow").is_none());
    }

    #[test]
    fn full_moons() {
        // 2026 年の満月（日本時間）。夜に唱えれば満月の夜になる
        for date in [
            "2026-01-03",
            "2026-03-03",
            "2026-05-31",
            "2026-08-28",
            "2026-09-26",
            "2026-12-24",
        ] {
            assert_eq!(
                at_str(&format!("{date} 21:00")),
                Some(Omen::FullMoon),
                "{date}"
            );
            // 昼は月が出ていない
            assert_eq!(at_str(&format!("{date} 12:00")), None, "{date}");
        }
        // 新月の夜は何も起きない
        assert_eq!(at_str("2026-09-11 21:00"), None);
    }

    #[test]
    fn special_days() {
        assert_eq!(at_str("2027-01-01 09:00"), Some(Omen::NewYear));
        assert_eq!(at_str("2026-07-07 20:00"), Some(Omen::Tanabata));
        assert_eq!(at_str("2026-07-07 12:00"), None);
        assert_eq!(at_str("2026-10-31 12:00"), Some(Omen::Halloween));
        assert_eq!(at_str("2026-11-13 12:00"), Some(Omen::Friday13));
        assert_eq!(at_str("2026-11-14 12:00"), None);
        assert_eq!(at_str("2026-09-24 02:15"), Some(Omen::Ushimitsu));
        assert_eq!(at_str("2026-09-24 02:45"), None);
        assert_eq!(at_str("2026-09-24 14:00"), None);
        for o in Omen::ALL {
            assert_eq!(Omen::from_key(o.key()), Some(o));
        }
    }
}
