mod circle;
mod config;
mod forbidden;
mod grimoire;
mod locale;
mod render;
mod script;
mod share;
mod shell;
mod terminal;

use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use std::time::Duration;

use circle::{MagicCircle, Tier};
use locale::{Locale, Source};

fn usage(l: Locale) -> String {
    let lines = [
        (
            "--explain",
            "魔法陣のハッシュとパラメータを表示する",
            "Show the circle's hash and parameters",
        ),
        (
            "--svg <file>",
            "魔法陣を SVG に書き出す",
            "Write the circle out as SVG",
        ),
        (
            "--no-run",
            "魔法陣だけ出して、コマンドは実行しない",
            "Unfold the circle but don't run the command",
        ),
        (
            "--share",
            "コマンドは実行せず、投稿文を stdout に、画像を今のディレクトリに出す",
            "Don't run the command; print a post to stdout and save an image here",
        ),
        (
            "--locale <ja|en>",
            "今回だけ表示の言語を変える",
            "Use this language for this run",
        ),
        (
            "--setup",
            "設定を表示する。--locale と一緒なら、その言語を保存する",
            "Show settings; with --locale, save that language",
        ),
        (
            "--grimoire",
            "図鑑を開く。これまでに集めた魔法陣の種類を見る",
            "Open the grimoire: see which kinds of circles you've collected",
        ),
        ("--help, -h", "この説明を表示する", "Show this help"),
    ];
    let mut s = String::from(
        "usage: maho [options] <command> [args...]
       maho [options] \"<shell command>\"
       maho --setup [--locale <ja|en>]
       maho --grimoire
       maho init <zsh|bash|fish>
       maho on | off
",
    );
    for (flag, ja, en) in lines {
        s.push_str(&format!("\n  {flag:<18} {}", l.pick(ja, en)));
    }
    s
}

/// `--share` で書き出す画像の一辺のピクセル数
const SHARE_PIXELS: u32 = 1200;

#[derive(Debug, Default, PartialEq)]
struct Options {
    explain: bool,
    share: bool,
    no_run: bool,
    /// 詠唱モードのフックから呼ばれた。`--no-run` に加えて、除外リストのコマンドを飛ばす
    chant: bool,
    setup: bool,
    grimoire: bool,
    help: bool,
    locale: Option<Locale>,
    svg_path: Option<String>,
    command: Vec<String>,
}

/// 引数の誤り。`--locale` を読み終えるまで表示の言語が決まらないので、文言は後で作る。
#[derive(Debug, PartialEq)]
enum ArgError {
    MissingValue(&'static str),
    UnknownLocale(String),
    UnknownOption(String),
    NoCommand,
    SetupWithCommand,
    GrimoireWithCommand,
}

impl ArgError {
    fn message(&self, l: Locale) -> String {
        match self {
            ArgError::MissingValue("--svg") => l
                .pick(
                    "--svg にはファイルパスが要ります",
                    "--svg needs a file path",
                )
                .into(),
            ArgError::MissingValue(flag) => match l {
                Locale::Ja => format!("{flag} には値が要ります"),
                Locale::En => format!("{flag} needs a value"),
            },
            ArgError::UnknownLocale(v) => match l {
                Locale::Ja => format!("知らない言語です: {v}（ja か en）"),
                Locale::En => format!("unknown locale: {v} (ja or en)"),
            },
            ArgError::UnknownOption(a) => match l {
                Locale::Ja => format!("知らないオプションです: {a}"),
                Locale::En => format!("unknown option: {a}"),
            },
            ArgError::NoCommand => l
                .pick("実行するコマンドがありません", "no command to run")
                .into(),
            ArgError::SetupWithCommand => l
                .pick(
                    "--setup はコマンドと一緒には使えません",
                    "--setup can't be used with a command",
                )
                .into(),
            ArgError::GrimoireWithCommand => l
                .pick(
                    "--grimoire はコマンドと一緒には使えません",
                    "--grimoire can't be used with a command",
                )
                .into(),
        }
    }
}

/// maho 自身のオプションはコマンドより前だけに置ける。
/// 最初のオプションでない引数（または `--` の次）から後ろは、すべてコマンドに渡す。
/// 誤りがあっても、そこまでに読んだ `--locale` はエラーの表示に使うので `opts` に残す。
fn parse_args(
    mut args: std::collections::VecDeque<String>,
    opts: &mut Options,
) -> Result<(), ArgError> {
    while let Some(arg) = args.front() {
        match arg.as_str() {
            "--explain" => opts.explain = true,
            "--share" => opts.share = true,
            "--no-run" => opts.no_run = true,
            // 詠唱モードのスクリプトだけが使う。usage には出さない
            "--chant" => {
                opts.chant = true;
                opts.no_run = true;
            }
            "--setup" => opts.setup = true,
            "--grimoire" => opts.grimoire = true,
            "--help" | "-h" => opts.help = true,
            "--svg" => {
                args.pop_front();
                let path = args.front().ok_or(ArgError::MissingValue("--svg"))?;
                opts.svg_path = Some(path.clone());
            }
            "--locale" => {
                args.pop_front();
                let v = args.front().ok_or(ArgError::MissingValue("--locale"))?;
                opts.locale =
                    Some(Locale::parse(v).ok_or_else(|| ArgError::UnknownLocale(v.clone()))?);
            }
            "--" => {
                args.pop_front();
                break;
            }
            a if a.starts_with("--") => return Err(ArgError::UnknownOption(a.into())),
            _ => break,
        }
        args.pop_front();
    }
    opts.command = args.into();
    if opts.help {
        return Ok(());
    }
    match (opts.setup, opts.grimoire, opts.command.is_empty()) {
        (true, _, false) => Err(ArgError::SetupWithCommand),
        (_, true, false) => Err(ArgError::GrimoireWithCommand),
        (false, false, true) => Err(ArgError::NoCommand),
        _ => Ok(()),
    }
}

fn main() -> ExitCode {
    let env = |k: &str| std::env::var(k).ok();
    let config_path = config::path(env);
    // 読めない設定ファイルは、言語が決まってから知らせて、既定の設定で続ける
    let (config, config_error) = match config_path.as_deref().map(config::Config::load) {
        Some(Err(e)) => (config::Config::default(), Some(e)),
        Some(Ok(c)) => (c, None),
        None => (config::Config::default(), None),
    };
    let mut opts = Options::default();
    let parsed = parse_args(std::env::args().skip(1).collect(), &mut opts);
    let (l, source) = match opts.locale {
        Some(l) => (l, Source::Flag),
        None => locale::detect(env, config.locale),
    };
    if let (Some(e), Some(path)) = (&config_error, &config_path) {
        match l {
            Locale::Ja => eprintln!(
                "maho: 設定ファイルを読めないので、既定の設定で続けます: {}\n{e}",
                path.display()
            ),
            Locale::En => eprintln!(
                "maho: can't read the config file, using the defaults: {}\n{e}",
                path.display()
            ),
        }
    }
    if let Err(e) = parsed {
        eprintln!("maho: {}\n{}", e.message(l), usage(l));
        return ExitCode::from(2);
    }
    if opts.help {
        println!("{}", usage(l));
        return ExitCode::SUCCESS;
    }
    if opts.setup {
        return setup(opts.locale, (l, source), config_path, &config);
    }
    let grimoire_path = grimoire::path(env);
    if opts.grimoire {
        return open_grimoire(grimoire_path, l);
    }
    let args = &opts.command;
    match args.iter().map(String::as_str).collect::<Vec<_>>()[..] {
        ["init", shell] => return init(shell, l),
        // シェルの関数が読み込まれていれば、ここへは来ない
        ["on"] | ["off"] => {
            eprintln!(
                "maho: {}",
                l.pick(
                    "maho on / off には、シェルの設定に maho init を書いてシェルを開き直してください（README の「詠唱モード」）",
                    "maho on / off needs maho init in your shell config; then open a new shell (see \"Chanting mode\" in the README)",
                )
            );
            return ExitCode::from(2);
        }
        _ => {}
    }

    // 魔法陣を決めるのは、ユーザーが打ったコマンド文字列そのもの。
    let spell = args.join(" ");
    if opts.chant && shell::skipped(&spell, &shell::skip_list(config.chant_skip.as_deref())) {
        return ExitCode::SUCCESS;
    }
    let mut circle = MagicCircle::from_command(&spell);
    if forbidden::is_forbidden(&spell) {
        circle.forbid();
    }
    // コマンドの stdout を汚さないよう、演出はすべて stderr に出す。
    // 描けなくてもコマンドは実行する。魔法陣は飾りでしかない。
    let target = terminal::detect(|k| std::env::var(k).ok(), terminal::ask_tmux);
    let frames = animation(&circle, &spell);
    let drawn = match terminal::play(&frames, FRAME_INTERVAL, target, circle.rows()) {
        Ok(drawn) => drawn,
        Err(e) => {
            match l {
                Locale::Ja => eprintln!("maho: 魔法陣を描けません: {e}"),
                Locale::En => eprintln!("maho: can't draw the magic circle: {e}"),
            }
            false
        }
    };
    if opts.explain {
        explain(&spell, &circle, l);
    } else if !drawn && std::io::stderr().is_terminal() {
        // 絵を出せない端末でも、魔法が発動したことだけは伝える
        eprintln!("{}", unfolded(&spell, &circle, l));
    } else if drawn && circle.forbidden {
        eprintln!("✦ {}", circle.title(l));
    } else if drawn && matches!(circle.tier, Tier::Large | Tier::Ultimate) {
        // 出にくい格を引いたときだけ、何が出たかを名乗る
        eprintln!("✦ {}", circle.tier.name(l));
    }
    if grimoire::enabled(env)
        && let Some(path) = &grimoire_path
    {
        record(path, &circle, l);
    }
    if let Some(path) = &opts.svg_path {
        if let Err(e) = std::fs::write(path, render::svg(&circle, &spell)) {
            match l {
                Locale::Ja => eprintln!("maho: 魔法陣を書き出せません: {path}: {e}"),
                Locale::En => eprintln!("maho: can't write the magic circle: {path}: {e}"),
            }
            return ExitCode::from(1);
        }
        if opts.explain {
            eprintln!("  svg       {path}");
        }
    }

    if opts.share {
        return share(&circle, &spell, l);
    }
    if opts.no_run {
        return ExitCode::SUCCESS;
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
            match l {
                Locale::Ja => eprintln!("maho: 詠唱失敗: {}: {e}", args[0]),
                Locale::En => eprintln!("maho: the spell failed: {}: {e}", args[0]),
            }
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
fn animation(c: &MagicCircle, spell: &str) -> Vec<String> {
    if std::env::var("MAHO_ANIMATION").as_deref() == Ok("off") {
        return vec![render::frame(c, spell, 1.0)];
    }
    (1..=FRAMES)
        .map(|i| render::frame(c, spell, i as f32 / FRAMES as f32))
        .collect()
}

/// 呪文を唱えずに、見せびらかす用の投稿文と画像だけを作る。
/// 投稿文は stdout に出すので、`maho --share git status | pbcopy` でそのまま貼れる。
fn share(c: &MagicCircle, spell: &str, l: Locale) -> ExitCode {
    let path = share::image_name(c);
    let png = terminal::rasterize(&render::frame(c, spell, 1.0), SHARE_PIXELS);
    if let Err(e) = png.and_then(|png| std::fs::write(&path, png).map_err(|e| e.to_string())) {
        match l {
            Locale::Ja => eprintln!("maho: 魔法陣の画像を書き出せません: {path}: {e}"),
            Locale::En => eprintln!("maho: can't write the magic circle image: {path}: {e}"),
        }
        return ExitCode::from(1);
    }
    let post = share::post(c, spell, l);
    println!("{post}");
    eprintln!("\n  {:<9} {path}", l.pick("画像", "image"));
    eprintln!(
        "  {:<9} {}",
        l.pick("X に投稿", "post on X"),
        share::intent_url(&post)
    );
    ExitCode::SUCCESS
}

/// 唱えた魔法陣を図鑑に書き込み、初めて見た項目があれば知らせる。
/// 図鑑も飾りなので、書けなくても黙ってコマンドを続ける。
fn record(path: &std::path::Path, c: &MagicCircle, l: Locale) {
    let Ok(mut g) = grimoire::Grimoire::load(path) else {
        return;
    };
    let new = g.record(c);
    if g.save(path).is_ok() && !new.is_empty() && std::io::stderr().is_terminal() {
        eprintln!("{}", grimoire::announce(&new, l));
    }
}

fn open_grimoire(path: Option<PathBuf>, l: Locale) -> ExitCode {
    let Some(path) = path else {
        eprintln!(
            "maho: {}",
            l.pick(
                "HOME が無いので図鑑の場所が決まりません",
                "can't find the grimoire: HOME is not set",
            )
        );
        return ExitCode::from(1);
    };
    match grimoire::Grimoire::load(&path) {
        Ok(g) => {
            print!("{}", grimoire::show(&g, l));
            ExitCode::SUCCESS
        }
        Err(e) => {
            match l {
                Locale::Ja => eprintln!("maho: 図鑑を読めません: {}: {e}", path.display()),
                Locale::En => eprintln!("maho: can't read the grimoire: {}: {e}", path.display()),
            }
            ExitCode::from(1)
        }
    }
}

fn init(shell: &str, l: Locale) -> ExitCode {
    let Some(script) = shell::init(shell, l) else {
        let known = shell::SHELLS.join(" / ");
        match l {
            Locale::Ja => eprintln!("maho: {shell} にはまだ対応していません（{known}）"),
            Locale::En => eprintln!("maho: {shell} isn't supported yet ({known})"),
        }
        return ExitCode::from(2);
    };
    print!("{script}");
    ExitCode::SUCCESS
}

/// `--setup`: 言語を渡されたら設定ファイルに保存し、無ければ今の設定を見せる。
fn setup(
    new: Option<Locale>,
    (l, source): (Locale, Source),
    path: Option<PathBuf>,
    config: &config::Config,
) -> ExitCode {
    let Some(path) = path else {
        eprintln!(
            "maho: {}",
            l.pick(
                "HOME が無いので設定ファイルの場所が決まりません",
                "can't find where to put the config file: HOME is not set",
            )
        );
        return ExitCode::from(1);
    };
    let Some(new) = new else {
        let from = match source {
            Source::Flag => "--locale",
            Source::Env => "MAHO_LOCALE",
            Source::Config => l.pick("設定ファイル", "config file"),
            Source::System => l.pick("LANG などの環境変数", "LANG and friends"),
            Source::Default => l.pick("既定", "default"),
        };
        let skip = shell::skip_list(config.chant_skip.as_deref()).join(" ");
        let skip_from = match config.chant_skip {
            Some(_) => l.pick("設定ファイル", "config file"),
            None => l.pick("既定", "default"),
        };
        println!("locale      {}  ({from})", l.code());
        println!("chant_skip  {skip}  ({skip_from})");
        println!("config      {}", path.display());
        return ExitCode::SUCCESS;
    };
    if let Err(e) = config::save_locale(&path, new) {
        match new {
            Locale::Ja => eprintln!("maho: 設定を保存できません: {}: {e}", path.display()),
            Locale::En => eprintln!("maho: can't save the config: {}: {e}", path.display()),
        }
        return ExitCode::from(1);
    }
    match new {
        Locale::Ja => println!("✦ 表示を日本語にしました（{}）", path.display()),
        Locale::En => println!("✦ Switched to English ({})", path.display()),
    }
    // 環境変数は設定ファイルより強いので、保存しても効かないことを知らせる
    if let Some(env) = std::env::var("MAHO_LOCALE")
        .ok()
        .as_deref()
        .and_then(Locale::parse)
        && env != new
    {
        eprintln!(
            "  {}",
            new.pick(
                "ただし MAHO_LOCALE が設定されているので、そちらが優先されます",
                "note: MAHO_LOCALE is set and takes precedence",
            )
        );
    }
    ExitCode::SUCCESS
}

fn unfolded(spell: &str, c: &MagicCircle, l: Locale) -> String {
    let what = if c.forbidden {
        l.pick("超極大魔法展開", "Super ultimate circle unfolded")
    } else {
        l.pick("魔法陣展開", "Magic circle unfolded")
    };
    format!("✦ {what}: {spell}")
}

fn explain(spell: &str, c: &MagicCircle, l: Locale) {
    eprintln!("{}", unfolded(spell, c, l));
    eprintln!("  hash      {}", c.hash_hex());
    eprintln!("  tier      {}", c.title(l));
    eprintln!(
        "  layout {}  shape {}  ornament {}  rings {}  symmetry {}  runes {}  particles {}",
        c.layout.name(l),
        c.shape.name(l),
        c.ornament.name(l),
        c.rings,
        c.symmetry,
        c.runes,
        c.particles
    );
    eprintln!(
        "  script {}  size {:.2}  spacing {:.2}  separator {}  band {}",
        c.hand.style.name(l),
        c.hand.size,
        c.hand.spacing,
        c.hand.separator.name(l),
        c.band.name(l)
    );
    eprintln!(
        "  rotation {:.1}°  hue {:.1}°  {}",
        c.rotation,
        c.hue,
        if c.clockwise {
            l.pick("右回り", "clockwise")
        } else {
            l.pick("左回り", "counterclockwise")
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

    fn parse(args: &[&str]) -> Result<Options, ArgError> {
        let mut opts = Options::default();
        parse_args(args.iter().map(|s| s.to_string()).collect(), &mut opts).map(|()| opts)
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
    fn share_flag() {
        let o = parse(&["--share", "git", "status"]).unwrap();
        assert!(o.share);
        assert_eq!(o.command, cmd(&["git", "status"]));
    }

    #[test]
    fn chant_flag_implies_no_run() {
        let o = parse(&["--chant", "--", "ls -la"]).unwrap();
        assert!(o.chant && o.no_run);
        assert_eq!(o.command, ["ls -la"]);
    }

    #[test]
    fn no_run_flag() {
        let o = parse(&["--no-run", "--", "ls | grep x"]).unwrap();
        assert!(o.no_run);
        assert_eq!(o.command, cmd(&["ls | grep x"]));
    }

    #[test]
    fn double_dash_ends_options() {
        let o = parse(&["--", "--explain"]).unwrap();
        assert_eq!(o.command, cmd(&["--explain"]));
    }

    #[test]
    fn locale_and_setup() {
        let o = parse(&["--locale", "en", "ls"]).unwrap();
        assert_eq!(o.locale, Some(Locale::En));
        let o = parse(&["--setup", "--locale", "ja"]).unwrap();
        assert!(o.setup);
        assert_eq!(o.locale, Some(Locale::Ja));
        assert!(parse(&["--setup"]).unwrap().command.is_empty());
        assert_eq!(parse(&["--setup", "ls"]), Err(ArgError::SetupWithCommand));
        assert!(parse(&["--grimoire"]).unwrap().grimoire);
        assert_eq!(
            parse(&["--grimoire", "ls"]),
            Err(ArgError::GrimoireWithCommand)
        );
        assert_eq!(
            parse(&["--locale", "fr", "ls"]),
            Err(ArgError::UnknownLocale("fr".into()))
        );
        assert_eq!(
            parse(&["--locale"]),
            Err(ArgError::MissingValue("--locale"))
        );
    }

    #[test]
    fn help() {
        assert!(parse(&["--help"]).unwrap().help);
        assert!(parse(&["-h"]).unwrap().help);
        assert!(parse(&["--locale", "en", "--help"]).unwrap().help);
        // コマンドの後ろの --help はコマンドのもの
        let o = parse(&["git", "--help"]).unwrap();
        assert!(!o.help);
        assert_eq!(o.command, cmd(&["git", "--help"]));
    }

    #[test]
    fn locale_survives_errors() {
        // 誤りの知らせも --locale の言語で出す
        for args in [&["--locale", "en"][..], &["--locale", "en", "--nope", "ls"]] {
            let mut opts = Options::default();
            let args = args.iter().map(|s| s.to_string()).collect();
            assert!(parse_args(args, &mut opts).is_err());
            assert_eq!(opts.locale, Some(Locale::En));
        }
    }

    #[test]
    fn errors() {
        assert!(parse(&[]).is_err());
        assert!(parse(&["--explain"]).is_err());
        assert!(parse(&["--svg"]).is_err());
        assert!(parse(&["--nope", "ls"]).is_err());
    }
}
