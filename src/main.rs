mod circle;
mod render;
mod terminal;

use std::io::IsTerminal;
use std::process::{Command, ExitCode};
use std::time::Duration;

use circle::MagicCircle;

const USAGE: &str = "usage: maho [--explain] [--svg <file>] <command> [args...]
       maho [--explain] [--svg <file>] \"<shell command>\"

  --explain     魔法陣のハッシュとパラメータを表示する
  --svg <file>  魔法陣を SVG に書き出す";

#[derive(Debug, Default, PartialEq)]
struct Options {
    explain: bool,
    svg_path: Option<String>,
    command: Vec<String>,
}

/// maho 自身のオプションはコマンドより前だけに置ける。
/// 最初のオプションでない引数（または `--` の次）から後ろは、すべてコマンドに渡す。
fn parse_args(mut args: std::collections::VecDeque<String>) -> Result<Options, String> {
    let mut opts = Options::default();
    while let Some(arg) = args.front() {
        match arg.as_str() {
            "--explain" => opts.explain = true,
            "--svg" => {
                args.pop_front();
                let path = args.front().ok_or("--svg にはファイルパスが要ります")?;
                opts.svg_path = Some(path.clone());
            }
            "--" => {
                args.pop_front();
                break;
            }
            a if a.starts_with("--") => return Err(format!("知らないオプションです: {a}")),
            _ => break,
        }
        args.pop_front();
    }
    opts.command = args.into();
    if opts.command.is_empty() {
        return Err("実行するコマンドがありません".into());
    }
    Ok(opts)
}

fn main() -> ExitCode {
    let opts = match parse_args(std::env::args().skip(1).collect()) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("maho: {e}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    let args = &opts.command;

    // 魔法陣を決めるのは、ユーザーが打ったコマンド文字列そのもの。
    let spell = args.join(" ");
    let circle = MagicCircle::from_command(&spell);
    // コマンドの stdout を汚さないよう、演出はすべて stderr に出す。
    // 描けなくてもコマンドは実行する。魔法陣は飾りでしかない。
    let protocol = terminal::detect(|k| std::env::var(k).ok());
    let drawn = match terminal::play(&animation(&circle), FRAME_INTERVAL, protocol) {
        Ok(drawn) => drawn,
        Err(e) => {
            eprintln!("maho: 魔法陣を描けません: {e}");
            false
        }
    };
    if opts.explain {
        explain(&spell, &circle);
    } else if !drawn && std::io::stderr().is_terminal() {
        // 絵を出せない端末でも、魔法が発動したことだけは伝える
        eprintln!("✦ 魔法陣展開: {spell}");
    }
    if let Some(path) = &opts.svg_path {
        if let Err(e) = std::fs::write(path, render::svg(&circle)) {
            eprintln!("maho: 魔法陣を書き出せません: {path}: {e}");
            return ExitCode::from(1);
        }
        if opts.explain {
            eprintln!("  svg       {path}");
        }
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

fn explain(spell: &str, c: &MagicCircle) {
    eprintln!("✦ 魔法陣展開: {spell}");
    eprintln!("  hash      {}", c.hash_hex());
    eprintln!(
        "  shape {}  ornament {}  rings {}  symmetry {}  runes {}  particles {}",
        c.shape.name(),
        c.ornament.name(),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Options, String> {
        parse_args(args.iter().map(|s| s.to_string()).collect())
    }

    fn cmd(args: &[&str]) -> Vec<String> {
        args.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn plain_command() {
        let o = parse(&["git", "status"]).unwrap();
        assert!(!o.explain);
        assert_eq!(o.command, cmd(&["git", "status"]));
    }

    #[test]
    fn options_in_any_order() {
        let o = parse(&["--svg", "a.svg", "--explain", "ls", "-la"]).unwrap();
        assert!(o.explain);
        assert_eq!(o.svg_path.as_deref(), Some("a.svg"));
        assert_eq!(o.command, cmd(&["ls", "-la"]));
    }

    #[test]
    fn options_after_command_belong_to_command() {
        let o = parse(&["cargo", "--explain", "E0308"]).unwrap();
        assert!(!o.explain);
        assert_eq!(o.command, cmd(&["cargo", "--explain", "E0308"]));
    }

    #[test]
    fn double_dash_ends_options() {
        let o = parse(&["--", "--explain"]).unwrap();
        assert_eq!(o.command, cmd(&["--explain"]));
    }

    #[test]
    fn errors() {
        assert!(parse(&[]).is_err());
        assert!(parse(&["--explain"]).is_err());
        assert!(parse(&["--svg"]).is_err());
        assert!(parse(&["--nope", "ls"]).is_err());
    }
}
