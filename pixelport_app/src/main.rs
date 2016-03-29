extern crate pixelport_document;
extern crate pixelport;
extern crate pixelport_viewport;
extern crate rustc_serialize;
extern crate docopt;
#[macro_use]
extern crate log;
extern crate env_logger;

use std::path::{Path, PathBuf};
use std::fs::File;
use std::io::prelude::*;
use std::env;

use pixelport_document::*;
use pixelport::*;

use docopt::Docopt;

const USAGE: &'static str = "
Pixelport

Usage:
  pixelport [options] [<document>]

Options:
  -h --help                Show this screen.
  --port=<pt>              TCP port to expose [default: 4303].
  --multisampling=<ms>     Multisampling [default: 8].
  --fullscreen             Fullscreen mode.
  --vsync                  Enable vsync.
  --headless               Headless mode.
  --width=<px>             Window width.
  --height=<px>            Window height.
  --fixedtimestep=<ms>     Fix the frame time step to x ms.
  --maxfps=<ms>            Max fps [default: 600].
  --genpondocs             Output Pon documentation to stdout and exit.
";

#[derive(Debug, RustcDecodable)]
pub struct Args {
    arg_document: Option<String>,
    flag_port: u16,
    flag_multisampling: u16,
    flag_fullscreen: bool,
    flag_vsync: bool,
    flag_headless: bool,
    flag_width: Option<u32>,
    flag_height: Option<u32>,
    flag_fixedtimestep: Option<u32>,
    flag_maxfps: Option<f32>,
    flag_genpondocs: bool,
}

fn main() {
    env_logger::init().unwrap();
    let args: Args = Docopt::new(USAGE)
                            .and_then(|d| d.decode())
                            .unwrap_or_else(|e| e.exit());

    let (doc, root_path) = {
        if let Some(filename) = args.arg_document {
            let path = Path::new(&filename);
            let root_path = path.parent().unwrap().to_path_buf();
            (DocumentDescription::FromFile(path.to_path_buf()), root_path)
        } else {
            (DocumentDescription::Empty, Path::new(".").to_path_buf())
        }
    };

    let mut app = App::new(AppOptions {
        viewport: pixelport_viewport::ViewportSubSystemOptions {
            fullscreen: args.flag_fullscreen,
            multisampling: args.flag_multisampling,
            vsync: args.flag_vsync,
            headless: args.flag_headless,
            window_size: if args.flag_width.is_some() && args.flag_height.is_some() {
                Some((args.flag_width.unwrap(), args.flag_height.unwrap()))
            } else {
                None
            }
        },
        port: args.flag_port,
        document: doc,
        root_path: root_path,
        time_progression: match args.flag_fixedtimestep {
            Some(v) => TimeProgression::Fixed { step_ms: v },
            None => TimeProgression::Real
        },
        min_frame_ms: match args.flag_maxfps {
            Some(v) => Some(1000.0 / v),
            None => None
        }
    });

    if args.flag_genpondocs {
        println!("{}", app.document.translater.generate_json_docs());
        return;
    }

    println!("## READY FOR CONNECTIONS ##");
    println!("{{ \"port\": {} }}", app.tcpinterface.port());

    info!("Starting main loop");
    while {
        app.update()
    } {}
    info!("Writing document to doc_state.xml");
    let mut f = File::create("doc_state.xml").unwrap();
    f.write_all(&app.document.to_string().into_bytes()).unwrap();
    info!("Done writing doc_state.xml");
}
