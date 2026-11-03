mod app;
mod audio;
mod autoeq;
mod config;
mod dsp;
mod i18n;
mod mixer;
mod presets;
mod recorder;
mod render;
mod routing;
mod theme;
mod tray;
mod widgets;

use std::sync::Arc;
use std::time::Duration;

use audio::{AudioParams, EngineHandle};
use config::Config;
use eframe::egui;
use routing::Routing;

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let no_route = args.iter().any(|a| a == "--no-route");
    let start_minimized = args.iter().any(|a| a == "--minimized");
    let quit_after = parse_duration(&args);

    if args.iter().any(|a| a == "--help" || a == "-h") {
        print_help();
        return Ok(());
    }

    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("eqLinux {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    if args.iter().any(|a| a == "--list-sinks") {
        match routing::list_sinks() {
            Ok(sinks) => {
                for sink in sinks {
                    println!("{}\t{}", sink.name, sink.description);
                }
            }
            Err(err) => eprintln!("{err}"),
        }
        return Ok(());
    }

    if let Some(index) = args.iter().position(|a| a == "--render") {
        let Some(input) = args.get(index + 1) else {
            eprintln!("--render requires an input WAV file and an output WAV file");
            std::process::exit(2);
        };
        let Some(output) = args.get(index + 2) else {
            eprintln!("--render requires an output WAV file");
            std::process::exit(2);
        };
        let config = Config::load();
        match render::render_file(
            std::path::Path::new(input),
            std::path::Path::new(output),
            &config.eq_params(),
            config.effective_volume(),
            config.balance,
        ) {
            Ok(()) => println!("Rendered {input} → {output}"),
            Err(err) => {
                eprintln!("render failed: {err}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    if args.iter().any(|a| a == "--diagnose") {
        match diagnose() {
            Ok(status) => println!("{status}"),
            Err(err) => {
                eprintln!("diagnose failed: {err}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([400.0, 720.0])
            .with_min_inner_size([400.0, 400.0])
            .with_decorations(false)
            .with_resizable(true)
            .with_title("eqLinux")
            .with_app_id("eqlinux")
            .with_icon(make_icon())
            .with_visible(!start_minimized),
        ..Default::default()
    };

    eframe::run_native(
        "eqLinux",
        options,
        Box::new(move |cc| {
            let spanish = Config::load().language == i18n::Language::Spanish;
            let tray_events = tray::start(cc.egui_ctx.clone(), spanish);
            if start_minimized {
                cc.egui_ctx
                    .send_viewport_cmd(egui::ViewportCommand::Visible(false));
                cc.egui_ctx
                    .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            Ok(Box::new(app::EqLinuxApp::new(
                cc,
                no_route,
                quit_after,
                tray_events,
            )))
        }),
    )
}

fn diagnose() -> Result<String, String> {
    let output = routing::default_sink()
        .filter(|name| !routing::is_virtual_sink(name))
        .or_else(|| {
            routing::list_sinks().ok().and_then(|sinks| {
                sinks
                    .into_iter()
                    .find(|sink| !routing::is_virtual_sink(&sink.name))
                    .map(|sink| sink.name)
            })
        })
        .ok_or_else(|| "no physical output sink".to_string())?;
    let mut routing = Routing::start("eqlinux_diag_sink")?;
    let engine = EngineHandle::start(
        routing.monitor_source().to_string(),
        output,
        AudioParams::default(),
    );

    std::thread::sleep(Duration::from_secs(2));
    let status = engine.status();
    drop(engine);
    routing.stop();
    Ok(status)
}

fn parse_duration(args: &[String]) -> Option<Duration> {
    let index = args.iter().position(|a| a == "--quit-after")?;
    let seconds = args.get(index + 1)?.parse::<f64>().ok()?;
    Some(Duration::from_secs_f64(seconds.max(0.1)))
}

fn print_help() {
    println!(
        "eqLinux {}\n\
         System-wide audio equalizer and volume mixer for Linux.\n\n\
         USAGE:\n    eqlinux [OPTIONS]\n\n\
         OPTIONS:\n\
             -h, --help         Show this help\n\
             -V, --version      Show version\n\
                 --minimized    Start with the window minimized\n\
                 --list-sinks   List available output devices\n\
                 --no-route     Preview the UI without touching system audio\n\
                 --quit-after N Exit after N seconds (useful for testing)\n\
                 --diagnose     Start routing and the audio engine, then report status\n\
                 --render IN OUT  Render a WAV file with the current settings\n\n\
         The application creates a virtual PulseAudio/PipeWire sink so that all\n\
         system audio can be equalized and then routed to the selected output.",
        env!("CARGO_PKG_VERSION")
    );
}

fn make_icon() -> Arc<egui::IconData> {
    Arc::new(
        eframe::icon_data::from_png_bytes(include_bytes!("../assets/logo.png"))
            .expect("embedded app logo must be a valid PNG"),
    )
}
