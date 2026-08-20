mod config;
mod host;
mod probe;
mod status;

use host::Target;
use probe::IcmpClients;
use slint::{ComponentHandle, Model, ModelRc, SharedString, Timer, TimerMode, VecModel};
use status::status_for;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

slint::include_modules!();

const VERSION: &str = env!("CARGO_PKG_VERSION");
const NAME: &str = env!("CARGO_PKG_NAME");

struct Site {
    target: Target,
    added_at: Instant,
    last_ok: Option<Instant>,
}

struct AppState {
    sites: Vec<Site>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(code) = handle_cli(std::env::args().skip(1)) {
        std::process::exit(code);
    }

    slint::set_xdg_app_id("edt-down-for-me").ok();

    let ui = AppWindow::new()?;
    ui.set_app_version(SharedString::from(VERSION));
    let model = std::rc::Rc::new(VecModel::<SiteRow>::default());
    ui.set_sites(ModelRc::from(model.clone()));

    let now = Instant::now();
    let initial = config::load();
    let state = Arc::new(Mutex::new(AppState {
        sites: initial
            .into_iter()
            .filter_map(|s| host::parse_target(&s).ok())
            .map(|target| Site {
                target,
                added_at: now,
                last_ok: None,
            })
            .collect(),
    }));
    let kick = Arc::new(tokio::sync::Notify::new());

    refresh_model(&model, &state);

    {
        let ui_weak = ui.as_weak();
        let model = model.clone();
        let state = state.clone();
        let kick = kick.clone();
        ui.on_add_site(move || {
            let Some(ui) = ui_weak.upgrade() else {
                return;
            };
            let raw = ui.get_new_host().to_string();
            match add_site(&state, &raw) {
                Ok(()) => {
                    ui.set_new_host(SharedString::from(""));
                    ui.set_notice(SharedString::from(""));
                    refresh_model(&model, &state);
                    persist(&state);
                    kick.notify_one();
                }
                Err(msg) => ui.set_notice(SharedString::from(msg)),
            }
        });
    }

    {
        let model = model.clone();
        let state = state.clone();
        ui.on_remove_site(move |index| {
            let idx = index as usize;
            let mut guard = state.lock().expect("state");
            if idx < guard.sites.len() {
                guard.sites.remove(idx);
            }
            drop(guard);
            refresh_model(&model, &state);
            persist(&state);
        });
    }

    let notice_timer = Timer::default();
    {
        let ui_weak = ui.as_weak();
        notice_timer.start(TimerMode::Repeated, Duration::from_secs(4), move || {
            if let Some(ui) = ui_weak.upgrade() {
                if !ui.get_notice().is_empty() {
                    ui.set_notice(SharedString::from(""));
                }
            }
        });
    }

    let status_timer = Timer::default();
    {
        let model = model.clone();
        let state = state.clone();
        status_timer.start(TimerMode::Repeated, Duration::from_millis(500), move || {
            refresh_model(&model, &state);
        });
    }

    start_probe_loop(state, kick);

    ui.run()?;
    drop(status_timer);
    drop(notice_timer);
    Ok(())
}

fn handle_cli<I, S>(args: I) -> Option<i32>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut show_help = false;
    let mut show_version = false;
    for arg in args {
        match arg.as_ref() {
            "-h" | "--help" => show_help = true,
            "-V" | "--version" => show_version = true,
            other => {
                eprintln!("{NAME}: unknown option '{other}'");
                eprint_help();
                return Some(2);
            }
        }
    }
    if show_help {
        print_help();
        return Some(0);
    }
    if show_version {
        println!("{NAME} {VERSION}");
        return Some(0);
    }
    None
}

fn print_help() {
    println!("{NAME} {VERSION}");
    println!(
        "Compact Linux GUI that shows whether configured web properties are accessible"
    );
    println!();
    println!("Usage: {NAME} [OPTIONS]");
    println!();
    println!("Options:");
    println!("  -h, --help     Show this help");
    println!("  -V, --version  Show version");
}

fn eprint_help() {
    eprintln!("Usage: {NAME} [OPTIONS]");
    eprintln!("Try '{NAME} --help' for more information.");
}

fn add_site(state: &Arc<Mutex<AppState>>, raw: &str) -> Result<(), String> {
    let target = host::parse_target(raw).map_err(|e| e.to_string())?;
    let mut guard = state.lock().expect("state");
    if guard
        .sites
        .iter()
        .any(|s| s.target.display.eq_ignore_ascii_case(&target.display))
    {
        return Err("Already listed".into());
    }
    guard.sites.push(Site {
        target,
        added_at: Instant::now(),
        last_ok: None,
    });
    Ok(())
}

fn persist(state: &Arc<Mutex<AppState>>) {
    let names: Vec<String> = state
        .lock()
        .expect("state")
        .sites
        .iter()
        .map(|s| s.target.display.clone())
        .collect();
    if let Err(err) = config::save(&names) {
        eprintln!("failed to save site list: {err}");
    }
}

fn refresh_model(model: &VecModel<SiteRow>, state: &Arc<Mutex<AppState>>) {
    let guard = state.lock().expect("state");
    let now = Instant::now();
    let rows: Vec<SiteRow> = guard
        .sites
        .iter()
        .map(|s| SiteRow {
            host: SharedString::from(s.target.display.as_str()),
            status: status_for(now, s.added_at, s.last_ok),
        })
        .collect();
    drop(guard);

    if model.row_count() != rows.len() {
        model.set_vec(rows);
        return;
    }
    for (i, row) in rows.into_iter().enumerate() {
        match model.row_data(i) {
            Some(existing) if existing.host == row.host && existing.status == row.status => {}
            _ => model.set_row_data(i, row),
        }
    }
}

fn start_probe_loop(state: Arc<Mutex<AppState>>, kick: Arc<tokio::sync::Notify>) {
    std::thread::Builder::new()
        .name("probes".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .expect("tokio runtime");
            rt.block_on(probe_loop(state, kick));
        })
        .expect("probe thread");
}

async fn probe_loop(state: Arc<Mutex<AppState>>, kick: Arc<tokio::sync::Notify>) {
    let icmp = IcmpClients::new();
    // Low-rate cycle: one probe burst per host about every 6 seconds.
    const CYCLE: Duration = Duration::from_secs(6);
    loop {
        let targets: Vec<Target> = state
            .lock()
            .expect("state")
            .sites
            .iter()
            .map(|s| s.target.clone())
            .collect();

        let mut set = tokio::task::JoinSet::new();
        for target in targets {
            let icmp = icmp.clone();
            let state = state.clone();
            set.spawn(async move {
                if probe::is_reachable(&icmp, &target).await {
                    mark_ok(&state, &target.display);
                }
            });
        }
        while set.join_next().await.is_some() {}
        tokio::select! {
            _ = tokio::time::sleep(CYCLE) => {}
            _ = kick.notified() => {}
        }
    }
}

fn mark_ok(state: &Arc<Mutex<AppState>>, display: &str) {
    let mut guard = state.lock().expect("state");
    if let Some(site) = guard
        .sites
        .iter_mut()
        .find(|s| s.target.display.eq_ignore_ascii_case(display))
    {
        site.last_ok = Some(Instant::now());
    }
}

#[cfg(test)]
mod cli_tests {
    use super::handle_cli;

    #[test]
    fn help_and_version_exit_zero() {
        assert_eq!(handle_cli(["--help"]), Some(0));
        assert_eq!(handle_cli(["-h"]), Some(0));
        assert_eq!(handle_cli(["--version"]), Some(0));
        assert_eq!(handle_cli(["-V"]), Some(0));
    }

    #[test]
    fn unknown_option_exits_two() {
        assert_eq!(handle_cli(["--nope"]), Some(2));
    }

    #[test]
    fn no_args_starts_gui() {
        assert_eq!(handle_cli(Vec::<&str>::new()), None);
    }
}
