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
        let mut tcpinterface = pixelport_tcpinterface::TCPInterfaceSubSystem::new(pixelport_tcpinterface::TCPInterfaceSubSystemOptions {
            port: opts.port,
            write_frames: true
        });
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
        tcpinterface.init(&mut translater);
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
        self.subdoc.on_cycle(&mut self.document, &cycle_changes, &mut self.models);
        self.template.on_cycle(&mut self.document, &cycle_changes);
        self.animation.on_cycle(&mut self.document, &cycle_changes, time, &mut self.models);
        self.layout.on_cycle(&mut self.document, &cycle_changes);
        self.picking.on_cycle(&mut self.document, &cycle_changes);
        self.viewport.on_cycle(&mut self.document, &cycle_changes, &mut self.resources, &mut self.models);
        self.tcpinterface.on_cycle(&mut self.document, &cycle_changes);
        self.culling.on_cycle(&mut self.document, &cycle_changes);

        self.animation.on_update(&mut self.document, time);
        self.layout.on_update(&mut self.document);
        self.picking.on_update(&mut self.document);
        self.viewport.on_update(&mut self.document, dtime, &mut self.resources, &mut self.models);
        let requests = self.tcpinterface.get_requests(&mut self.document);
        for req in requests {
            let resp = self.handle_request((*req.request).bus_value_clone(), req.socket_token);
            self.tcpinterface.send_response(req.request_id, req.socket_token, resp);
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
    pub fn handle_request(&mut self, request: Box<BusValue>, socket_token: SocketToken)
            -> Result<Box<OutMessage>, RequestError> {
        if let Some(resp) = document_handle_request((*request).bus_value_clone(), socket_token, &mut self.document) {
            return resp;
        } else if let Some(resp) = self.tcpinterface.handle_request((*request).bus_value_clone(), socket_token, &mut self.document) {
            return resp;
        } else if let Some(resp) = self.viewport.handle_request((*request).bus_value_clone(), socket_token, &mut self.document, &mut self.resources, &mut self.models) {
            return resp;
        }

        Err(RequestError {
            request_id: "".to_string(),
            error_type: RequestErrorType::BadRequest,
            message: format!("No handler available for {:?}", request)
        })
    }
}
//
// struct TCPInterfaceEnvironment<'a> {
//     viewport: &'a mut pixelport_viewport::ViewportSubSystem,
//     resources: &'a mut pixelport_resources::ResourceStorage,
//     models: &'a mut pixelport_models::Models,
// }
//
// impl<'a> pixelport_tcpinterface::ITCPInterfaceEnvironment for TCPInterfaceEnvironment<'a> {
//     fn window_events(&self) -> Vec<glutin::Event> {
//         self.viewport.window_events.iter().map(|x| x.clone()).collect()
//     }
//     fn dump_resources(&self) {
//         self.resources.dump();
//     }
//     fn entity_renderers_bounding(&mut self, entity_id: EntityId, doc: &mut Document) -> Result<HashMap<String, pixelport_tcpinterface::AABB>, String> {
//         match self.viewport.entity_renderers_bounding(self.resources, &mut self.models, entity_id, doc) {
//             Ok(boundings) => Ok(boundings.into_iter().map(|(renderer_name, bounding)| {
//                 (renderer_name, pixelport_tcpinterface::AABB {
//                     screen_min: pixelport_tcpinterface::Vec3 {
//                         x: self.viewport.current_window_size.0 as f32 * (bounding.min.x + 1.0) / 2.0,
//                         y: self.viewport.current_window_size.1 as f32 * (bounding.min.y + 1.0) / 2.0,
//                         z: bounding.min.z
//                     },
//                     screen_max: pixelport_tcpinterface::Vec3 {
//                         x: self.viewport.current_window_size.0 as f32 * (bounding.max.x + 1.0) / 2.0,
//                         y: self.viewport.current_window_size.1 as f32 * (bounding.max.y + 1.0) / 2.0,
//                         z: bounding.max.z
//                     },
//                     viewport_min: bounding.min.into(),
//                     viewport_max: bounding.max.into(),
//                 })
//             }).collect()),
//             Err(err) => Err(err)
//         }
//     }
//     fn set_visualize_entity_bounding(&mut self, entity_id: Option<EntityId>) {
//         self.viewport.visualize_entity_bounding = entity_id;
//     }
//     fn fake_window_event(&mut self, event: glutin::Event) {
//         self.viewport.fake_window_event(event);
//     }
//     fn get_renderer_stats(&mut self) -> Vec<pixelport_tcpinterface::messages::RendererStats> {
//         let mut stats = vec![];
//         for r in &self.viewport.renderers {
//             stats.push(pixelport_tcpinterface::messages::RendererStats {
//                 name: r.desc.name.clone(),
//                 n_renderables: r.n_renderables()
//             })
//         }
//         stats
//     }
//     fn list_textures(&mut self) -> Vec<pixelport_tcpinterface::messages::Texture> {
//         self.resources.gl_textures.iter().map(|(k, v)| {
//             pixelport_tcpinterface::messages::Texture {
//                 name: format!("{:?}", k),
//                 id: match v.value() {
//                     Some(v) => v.texture,
//                     None => 0
//                 }
//             }
//         }).collect()
//     }
//     fn get_texture_content(&mut self, id: u32) -> Result<pixelport_tcpinterface::messages::RawImage, String> {
//         let t = self.resources.gl_textures.values().find(|v| {
//             if let Some(v) = v.value() {
//                 v.texture == id
//             } else {
//                 false
//             }
//         });
//         if let Some(t) = t {
//             if let Some(t) = t.value() {
//                 let ts = t.to_texture_source();
//                 Ok(pixelport_tcpinterface::messages::RawImage {
//                     content: ts.to_base64().unwrap(),
//                     width: t.width as u32,
//                     height: t.height as u32,
//                     pixel_format: t.format.to_pon_enum(),
//                     pixel_type: ts.content.pixel_type().to_pon_enum()
//                 })
//             } else {
//                 Err(format!("Texture still loading: {}", id))
//             }
//         } else {
//             Err(format!("No such texture: {}", id))
//         }
//     }
//     fn await_all_resources(&mut self) {
//         self.resources.await_all();
//     }
// }

#[no_mangle]
pub extern "C" fn pixelport_new() -> *mut App {
    let app = Box::new(App::new(AppOptions {
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
    }));
    unsafe { mem::transmute(app) }
}

#[no_mangle]
pub extern "C" fn pixelport_update(app: &mut App) -> bool { app.update() }

#[no_mangle]
pub extern "C" fn pixelport_request(app: &mut App, request: *mut c_char) {
    let request = unsafe { CStr::from_ptr(request).to_string_lossy().into_owned() };
    match Pon::from_string(&request) {
        Ok(pon) => match app.document.translater.translate_raw(&pon, &mut app.document.bus) {
            Ok(request) => {
                let resp = app.handle_request(request, 0);
            },
            Err(err) => { println!("Request translate error: {:?}", err); }
        },
        Err(err) => { println!("Request parse error: {:?}", err); }
    }
}
