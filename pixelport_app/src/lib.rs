extern crate pixelport_document;
extern crate pixelport_std;
extern crate pixelport_animation;
extern crate pixelport_viewport;
extern crate pixelport_template;
extern crate pixelport_subdoc;
extern crate pixelport_tcpinterface;
extern crate pixelport_picking;
extern crate pixelport_layout;
extern crate pixelport_resources;
extern crate pixelport_bounding;
extern crate pixelport_culling;
extern crate pixelport_models;
#[macro_use]
extern crate log;
extern crate time;
extern crate glutin;
extern crate mesh;
extern crate libc;

use std::mem;
use std::ffi::CStr;
use libc::c_char;
use std::path::{PathBuf, Path};
use time::*;
use std::rc::Rc;
use std::cell::RefCell;
use std::collections::HashMap;

use pixelport_document::*;

#[repr(C)]
pub struct App {
    pub document: Document,
    pub document_channels: DocumentChannels,
    pub subdoc: pixelport_subdoc::SubdocSubSystem,
    pub template: pixelport_template::TemplateSubSystem,
    pub animation: pixelport_animation::AnimationSubSystem,
    pub viewport: pixelport_viewport::ViewportSubSystem,
    pub tcpinterface: pixelport_tcpinterface::TCPInterfaceSubSystem,
    pub picking: pixelport_picking::PickingSubSystem,
    pub culling: pixelport_culling::CullingSubSystem,
    pub layout: pixelport_layout::LayoutSubSystem,
    pub resources: pixelport_resources::ResourceStorage,
    pub models: pixelport_models::Models,
    start_time: Timespec,
    prev_time: Timespec,
    time_progression: TimeProgression,
    min_frame_ms: Option<f32>,
}

#[derive(PartialEq)]
pub enum TimeProgression {
    Real,
    Fixed { step_ms: u32 }
}

pub enum DocumentDescription {
    Empty,
    FromFile(PathBuf)
}

pub struct AppOptions {
    pub viewport: pixelport_viewport::ViewportSubSystemOptions,
    pub port: u16,
    pub document: DocumentDescription,
    pub root_path: PathBuf,
    pub time_progression: TimeProgression,
    pub min_frame_ms: Option<f32>
}

impl App {
    pub fn new(mut opts: AppOptions) -> App {
        let mut subdoc = pixelport_subdoc::SubdocSubSystem::new();
        let mut template = pixelport_template::TemplateSubSystem::new(opts.root_path.clone());
        let mut animation = pixelport_animation::AnimationSubSystem::new();
        let mut resources = pixelport_resources::ResourceStorage::new(opts.root_path.clone());
        let mut viewport = pixelport_viewport::ViewportSubSystem::new(opts.root_path.clone(), &opts.viewport, &mut resources);
        let mut tcpinterface = pixelport_tcpinterface::TCPInterfaceSubSystem::new(opts.port);
        let mut picking = pixelport_picking::PickingSubSystem::new();
        let mut culling = pixelport_culling::CullingSubSystem::new();
        let mut layout = pixelport_layout::LayoutSubSystem::new();
        let mut models = pixelport_models::Models::new(opts.root_path.clone());

        let mut translater = PonTranslater::new();
        pixelport_std::pon_std(&mut translater);
        pixelport_bounding::pon_bounding(&mut translater);
        pixelport_models::pon_models(&mut translater);
        pixelport_models::init_logging();
        subdoc.on_init(&mut translater);
        template.on_init(&mut translater);
        animation.on_init(&mut translater);
        viewport.on_init(&mut translater, &mut template);
        picking.on_init(&mut translater);
        culling.on_init(&mut translater);
        layout.on_init(&mut translater);
        pon_document_requests(&mut translater);

        let start_time = match &opts.time_progression {
            &TimeProgression::Real => time::get_time(),
            &TimeProgression::Fixed { .. } => Timespec::new(0, 0)
        };
        let start_time_inner = start_time.clone();
        translater.register_function(move |_, _, _| {
            let t: f32 = (time::get_time() - start_time_inner).num_milliseconds() as f32 / 1000.0;
            Ok(Box::new(t))
        }, PonDocFunction {
            name: "time".to_string(),
            target_type_name: "f32".to_string(),
            arg: PonDocMatcher::Nil,
            module: "Utils".to_string(),
            doc: "Get the current time".to_string()
        });

        let mut document = match &opts.document {
            &DocumentDescription::Empty => Document::new_with_root(translater),
            &DocumentDescription::FromFile(ref path) => Document::from_file(translater, path).unwrap()
        };

        viewport.set_doc(&mut document);

        App {
            document: document,
            document_channels: DocumentChannels::new(),
            subdoc: subdoc,
            template: template,
            animation: animation,
            viewport: viewport,
            tcpinterface: tcpinterface,
            picking: picking,
            culling: culling,
            layout: layout,
            resources: resources,
            models: models,
            start_time: start_time,
            prev_time: start_time,
            time_progression: opts.time_progression,
            min_frame_ms: opts.min_frame_ms,
        }
    }

    pub fn update(&mut self) -> bool {
        let curr_time = match &self.time_progression {
            &TimeProgression::Real => time::get_time(),
            &TimeProgression::Fixed { ref step_ms } => self.prev_time + Duration::milliseconds(*step_ms as i64)
        };
        let time = curr_time - self.start_time;
        let dtime = curr_time - self.prev_time;
        self.prev_time = curr_time;

        if self.viewport.pre_update(&mut self.document) { return false; }

        let cycle_changes = self.document.close_cycle();
        for outbound_message in self.document_channels.cycle_changes(&mut self.document, &cycle_changes) {
            self.tcpinterface.send_message(outbound_message);
        }
        self.subdoc.on_cycle(&mut self.document, &cycle_changes, &mut self.models);
        self.template.on_cycle(&mut self.document, &cycle_changes);
        self.animation.on_cycle(&mut self.document, &cycle_changes, time, &mut self.models);
        self.layout.on_cycle(&mut self.document, &cycle_changes);
        self.picking.on_cycle(&mut self.document, &cycle_changes);
        self.viewport.on_cycle(&mut self.document, &cycle_changes, &mut self.resources, &mut self.models);
        self.culling.on_cycle(&mut self.document, &cycle_changes);

        self.animation.on_update(&mut self.document, time);
        self.layout.on_update(&mut self.document);
        self.picking.on_update(&mut self.document);
        self.viewport.on_update(&mut self.document, dtime, &mut self.resources, &mut self.models);
        for outbound_message in self.viewport.get_outbound_messages() {
            self.tcpinterface.send_message(outbound_message);
        }
        let requests = self.tcpinterface.get_requests(&mut self.document);
        for req in requests {
            let messages = self.handle_request(req);
            for message in messages {
                self.tcpinterface.send_message(message);
            }
        }
        self.culling.on_update(&mut self.document);
        self.resources.update();
        if let Some(min_frame_ms) = self.min_frame_ms {
            let dtime = time::get_time() - self.prev_time;
            if (dtime.num_milliseconds() as f32) < min_frame_ms {
                let sleep = min_frame_ms - dtime.num_milliseconds() as f32;
                ::std::thread::sleep(::std::time::Duration::from_millis(sleep as u64));
            }
        }
        return true;
    }
    pub fn handle_request(&mut self, request: IncomingMessage) -> Vec<OutgoingMessage> {
        let mut msgs = Vec::new();
        msgs.extend(self.document_channels.handle_request(&request, &mut self.document));
        msgs.extend(self.viewport.handle_request(&request, &mut self.document, &mut self.resources, &mut self.models));
        msgs
    }
}

#[repr(C)]
pub struct CApp {
    app: App,
    request_counter: u64
}

#[no_mangle]
pub extern "C" fn pixelport_new() -> *mut CApp {
    let app = Box::new(CApp {
        app: App::new(AppOptions {
            viewport: pixelport_viewport::ViewportSubSystemOptions {
                fullscreen: false,
                multisampling: 0,
                vsync: false,
                headless: false,
                window_size: None
            },
            port: 4303,
            document: DocumentDescription::Empty,
            root_path: Path::new(".").to_path_buf(),
            time_progression: TimeProgression::Real,
            min_frame_ms: None
        }),
        request_counter: 0
    });
    unsafe { mem::transmute(app) }
}

#[no_mangle]
pub extern "C" fn pixelport_update(app: &mut CApp) -> bool { app.app.update() }

#[no_mangle]
pub extern "C" fn pixelport_request(app: &mut CApp, request: *mut c_char) -> u64 {
    app.request_counter += 1;
    let channel_id = format!("{}", app.request_counter);
    let request = unsafe { CStr::from_ptr(request).to_string_lossy().into_owned() };
    match IncomingMessage::from_string(&app.app.document.translater, &mut app.app.document.bus, ClientId::CAPI, channel_id, &request) {
        Ok(request) => { app.app.handle_request(request); },
        Err(err) => unimplemented!()
    }
    app.request_counter
}
