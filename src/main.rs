mod circle;
mod render;
mod terminal;

use std::process::{Command, ExitCode};
use std::time::Duration;

use circle::MagicCircle;

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    // maho 自身のオプションはコマンドより前だけに置ける。以降はすべてコマンドに渡す。
    let mut svg_path = None;
    if args.first().map(String::as_str) == Some("--svg") {
        if args.len() < 2 {
            eprintln!("maho: --svg にはファイルパスが要ります");
            return ExitCode::from(2);
        }
        svg_path = Some(args.remove(1));
        args.remove(0);
    }
    if args.is_empty() {
        eprintln!("usage: maho [--svg <file>] <command> [args...]");
        eprintln!("       maho [--svg <file>] \"<shell command>\"");
        return ExitCode::from(2);
    }

    // 魔法陣を決めるのは、ユーザーが打ったコマンド文字列そのもの。
    let spell = args.join(" ");
    let circle = MagicCircle::from_command(&spell);
    let svg = render::svg(&circle);
    // コマンドの stdout を汚さないよう、演出はすべて stderr に出す。
    // 描けなくてもコマンドは実行する。魔法陣は飾りでしかない。
    let protocol = terminal::detect(|k| std::env::var(k).ok());
    if let Err(e) = terminal::play(&animation(&circle), FRAME_INTERVAL, protocol) {
        eprintln!("maho: 魔法陣を描けません: {e}");
    }
    announce(&spell, &circle);
    if let Some(path) = &svg_path {
        if let Err(e) = std::fs::write(path, &svg) {
            eprintln!("maho: 魔法陣を書き出せません: {path}: {e}");
            return ExitCode::from(1);
        }
        eprintln!("  svg       {path}");
    }

    // 引数 1 つで空白を含むなら `maho "cargo build && ls"` の形とみなしてシェルに渡す。
    let mut cmd = if args.len() == 1 && args[0].contains(char::is_whitespace) {
        let mut c = Command::new("sh");
        c.arg("-c").arg(&args[0]);
        c
    } else {
        let mut c = Command::new(&args[0]);
        c.args(&args[1..]);
        c
    };

    match cmd.status() {
        Ok(status) => exit_code(status),
        Err(e) => {
            eprintln!("maho: 詠唱失敗: {}: {e}", args[0]);
            ExitCode::from(if e.kind() == std::io::ErrorKind::NotFound {
                127
            } else {
                126
            })
        }
    }
}

/// 展開アニメーションのコマ数と間隔。合わせて 0.7 秒ほどで、待たされる感じを出さない。
const FRAMES: u32 = 16;
const FRAME_INTERVAL: Duration = Duration::from_millis(45);

/// `MAHO_ANIMATION=off` なら完成図 1 枚だけにする。
fn animation(c: &MagicCircle) -> Vec<String> {
    if std::env::var("MAHO_ANIMATION").as_deref() == Ok("off") {
        return vec![render::frame(c, 1.0)];
    }
    (1..=FRAMES)
        .map(|i| render::frame(c, i as f32 / FRAMES as f32))
        .collect()
}

fn announce(spell: &str, c: &MagicCircle) {
    eprintln!("✦ 魔法陣展開: {spell}");
    eprintln!("  hash      {}", c.hash_hex());
    eprintln!(
        "  shape {}  rings {}  symmetry {}  runes {}  particles {}",
        c.shape.name(),
        c.rings,
        c.symmetry,
        c.runes,
        c.particles
    );
    eprintln!(
        "  rotation {:.1}°  hue {:.1}°  {}",
        c.rotation,
        c.hue,
        if c.clockwise {
            "右回り"
        } else {
            "左回り"
        }
    );
}

/// 子プロセスの終了コードをそのまま返す。シグナルで死んだら 128+signal。
fn exit_code(status: std::process::ExitStatus) -> ExitCode {
    if let Some(code) = status.code() {
        return ExitCode::from(code as u8);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(sig) = status.signal() {
            return ExitCode::from((128 + sig) as u8);
        }
    }
    ExitCode::FAILURE
}
