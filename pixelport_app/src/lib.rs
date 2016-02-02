extern crate pixelport_document;
extern crate pixelport_util;
extern crate pixelport_animation;
extern crate pixelport_viewport;
extern crate pixelport_template;
extern crate pixelport_subdoc;
extern crate pixelport_tcpinterface;
extern crate pixelport_picking;
extern crate pixelport_layout;
extern crate pixelport_resources;
#[macro_use]
extern crate log;
extern crate time;
extern crate glutin;
extern crate mesh;

use std::path::{PathBuf, Path};
use time::*;

use pixelport_document::*;

pub struct App {
    pub document: Document,
    pub subdoc: pixelport_subdoc::SubdocSubSystem,
    pub template: pixelport_template::TemplateSubSystem,
    pub animation: pixelport_animation::AnimationSubSystem,
    pub viewport: pixelport_viewport::ViewportSubSystem,
    pub tcpinterface: pixelport_tcpinterface::TCPInterfaceSubSystem,
    pub picking: pixelport_picking::PickingSubSystem,
    pub layout: pixelport_layout::LayoutSubSystem,
    pub resources: pixelport_resources::ResourceStorage,
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

pub struct AppOptions {
    pub viewport: pixelport_viewport::ViewportSubSystemOptions,
    pub port: u16,
    pub document: Document,
    pub root_path: PathBuf,
    pub time_progression: TimeProgression,
    pub min_frame_ms: Option<f32>
}

impl App {
    pub fn new(mut opts: AppOptions) -> App {
        let mut subdoc = pixelport_subdoc::SubdocSubSystem::new(opts.root_path.clone());
        let mut template = pixelport_template::TemplateSubSystem::new(opts.root_path.clone());
        let mut animation = pixelport_animation::AnimationSubSystem::new();
        let mut resources = pixelport_resources::ResourceStorage::new(opts.root_path.clone());
        let mut viewport = pixelport_viewport::ViewportSubSystem::new(opts.root_path.clone(), &opts.viewport, &mut resources);
        let mut tcpinterface = pixelport_tcpinterface::TCPInterfaceSubSystem::new(opts.port);
        let mut picking = pixelport_picking::PickingSubSystem::new();
        let mut layout = pixelport_layout::LayoutSubSystem::new();

        pixelport_util::pon_util(&mut opts.document.runtime);
        subdoc.on_init(&mut opts.document);
        template.on_init(&mut opts.document);
        animation.on_init(&mut opts.document);
        viewport.on_init(&mut opts.document);
        picking.on_init(&mut opts.document);
        layout.on_init(&mut opts.document);

        println!("## READY FOR CONNECTIONS ##");
        println!("{{ \"port\": {} }}", tcpinterface.port());
        let start_time = match &opts.time_progression {
            &TimeProgression::Real => time::get_time(),
            &TimeProgression::Fixed { .. } => Timespec::new(0, 0)
        };
        App {
            document: opts.document,
            subdoc: subdoc,
            template: template,
            animation: animation,
            viewport: viewport,
            tcpinterface: tcpinterface,
            picking: picking,
            layout: layout,
            resources: resources,
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

        for _ in 0..100 { // At most 100 cycles before we do an update
            // A cycle is basically a subset of a frame. There might be 1 or more cycles per frame.
            let cycle_changes = self.document.close_cycle();
            self.subdoc.on_cycle(&mut self.document, &cycle_changes);
            self.template.on_cycle(&mut self.document, &cycle_changes);
            self.animation.on_cycle(&mut self.document, &cycle_changes);
            self.layout.on_cycle(&mut self.document, &cycle_changes);
            self.viewport.on_cycle(&mut self.document, &cycle_changes, &mut self.resources);
            self.tcpinterface.on_cycle(&mut self.document, &cycle_changes);
            self.picking.on_cycle(&mut self.document, &cycle_changes);
            if cycle_changes.set_properties.len() == 0 && cycle_changes.entities_added.len() == 0 &&
                cycle_changes.entities_removed.len() == 0 {
                break;
            }
        }
        self.subdoc.on_update(&mut self.document);
        self.animation.on_update(&mut self.document, time);
        self.layout.on_update(&mut self.document);
        if self.viewport.on_update(&mut self.document, dtime, &mut self.resources) { return false; }
        self.tcpinterface.on_update(&mut self.document, &mut TCPInterfaceEnvironment { resources: &mut self.resources, viewport: &mut self.viewport });
        self.picking.on_update(&mut self.document);
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
}

struct TCPInterfaceEnvironment<'a> {
    viewport: &'a mut pixelport_viewport::ViewportSubSystem,
    resources: &'a mut pixelport_resources::ResourceStorage
}

impl<'a> pixelport_tcpinterface::ITCPInterfaceEnvironment for TCPInterfaceEnvironment<'a> {
    fn window_events(&self) -> Vec<glutin::Event> {
        self.viewport.window_events.iter().map(|x| x.clone()).collect()
    }
    fn screenshot_to_png_data(&self) -> Result<Vec<u8>, String> {
        match self.viewport.screenshot() {
            Ok(ts) => {
                let mut png_data = Vec::new();
                ts.write_png(&mut png_data);
                Ok(png_data)
            },
            Err(err) => Err(format!("Failed to create screenshot: {:?}", err))
        }
    }
    fn screenshot_to_file(&self, path: &str) -> Result<(), String> {
        match self.viewport.screenshot() {
            Ok(ts) => {
                match ts.save(Path::new(&path)) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(format!("Failed to save screenshot: {:?}", err))
                }
            },
            Err(err) => Err(format!("Failed to create screenshot: {:?}", err))
        }
    }
    fn rebuild_scene(&mut self, doc: &mut Document) {
        self.viewport.rebuild_scene(self.resources, doc);
    }
    fn update_all_uniforms(&mut self, doc: &mut Document) {
        self.viewport.update_all_uniforms(doc);
    }
    fn dump_pipeline(&self) {
        self.viewport.dump_pipeline();
    }
    fn dump_resources(&self) {
        self.resources.dump();
    }
    fn entity_renderers_bounding(&mut self, entity_id: EntityId, doc: &mut Document) -> Result<Vec<mesh::AABB3>, String> {
        self.viewport.entity_renderers_bounding(self.resources, entity_id, doc)
    }
    fn set_visualize_entity_bounding(&mut self, entity_id: Option<EntityId>) {
        self.viewport.visualize_entity_bounding = entity_id;
    }
    fn fake_window_event(&mut self, event: glutin::Event) {
        self.viewport.fake_window_event(event);
    }
}
