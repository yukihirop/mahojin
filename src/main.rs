mod circle;

use std::process::{Command, ExitCode};

use circle::MagicCircle;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("usage: maho <command> [args...]");
        eprintln!("       maho \"<shell command>\"");
        return ExitCode::from(2);
    }

    // 魔法陣を決めるのは、ユーザーが打ったコマンド文字列そのもの。
    let spell = args.join(" ");
    let circle = MagicCircle::from_command(&spell);
    // コマンドの stdout を汚さないよう、演出はすべて stderr に出す。
    announce(&spell, &circle);

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
