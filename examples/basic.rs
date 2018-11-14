/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

extern crate app_units;
extern crate euclid;
extern crate gleam;
extern crate glutin;
extern crate webrender;
extern crate winit;

#[path = "common/boilerplate.rs"]
mod boilerplate;

use boilerplate::{Example, HandyDandyRectBuilder};
use winit::TouchPhase;
use std::collections::HashMap;
use webrender::ShaderPrecacheFlags;
use webrender::api::*;

#[derive(Debug)]
enum Gesture {
    None,
    Pan,
    Zoom,
}

#[derive(Debug)]
struct Touch {
    id: u64,
    start_x: f32,
    start_y: f32,
    current_x: f32,
    current_y: f32,
}

fn dist(x0: f32, y0: f32, x1: f32, y1: f32) -> f32 {
    let dx = x0 - x1;
    let dy = y0 - y1;
    ((dx * dx) + (dy * dy)).sqrt()
}

impl Touch {
    fn distance_from_start(&self) -> f32 {
        dist(self.start_x, self.start_y, self.current_x, self.current_y)
    }

    fn initial_distance_from_other(&self, other: &Touch) -> f32 {
        dist(self.start_x, self.start_y, other.start_x, other.start_y)
    }

    fn current_distance_from_other(&self, other: &Touch) -> f32 {
        dist(
            self.current_x,
            self.current_y,
            other.current_x,
            other.current_y,
        )
    }
}

struct TouchState {
    active_touches: HashMap<u64, Touch>,
    current_gesture: Gesture,
    start_zoom: f32,
    current_zoom: f32,
    start_pan: DeviceIntPoint,
    current_pan: DeviceIntPoint,
}

enum TouchResult {
    None,
    Pan(DeviceIntPoint),
    Zoom(f32),
}

impl TouchState {
    fn new() -> TouchState {
        TouchState {
            active_touches: HashMap::new(),
            current_gesture: Gesture::None,
            start_zoom: 1.0,
            current_zoom: 1.0,
            start_pan: DeviceIntPoint::zero(),
            current_pan: DeviceIntPoint::zero(),
        }
    }

    fn handle_event(&mut self, touch: winit::Touch) -> TouchResult {
        match touch.phase {
            TouchPhase::Started => {
                debug_assert!(!self.active_touches.contains_key(&touch.id));
                self.active_touches.insert(
                    touch.id,
                    Touch {
                        id: touch.id,
                        start_x: touch.location.x as f32,
                        start_y: touch.location.y as f32,
                        current_x: touch.location.x as f32,
                        current_y: touch.location.y as f32,
                    },
                );
                self.current_gesture = Gesture::None;
            }
            TouchPhase::Moved => {
                match self.active_touches.get_mut(&touch.id) {
                    Some(active_touch) => {
                        active_touch.current_x = touch.location.x as f32;
                        active_touch.current_y = touch.location.y as f32;
                    }
                    None => panic!("move touch event with unknown touch id!"),
                }

                match self.current_gesture {
                    Gesture::None => {
                        let mut over_threshold_count = 0;
                        let active_touch_count = self.active_touches.len();

                        for (_, touch) in &self.active_touches {
                            if touch.distance_from_start() > 8.0 {
                                over_threshold_count += 1;
                            }
                        }

                        if active_touch_count == over_threshold_count {
                            if active_touch_count == 1 {
                                self.start_pan = self.current_pan;
                                self.current_gesture = Gesture::Pan;
                            } else if active_touch_count == 2 {
                                self.start_zoom = self.current_zoom;
                                self.current_gesture = Gesture::Zoom;
                            }
                        }
                    }
                    Gesture::Pan => {
                        let keys: Vec<u64> = self.active_touches.keys().cloned().collect();
                        debug_assert!(keys.len() == 1);
                        let active_touch = &self.active_touches[&keys[0]];
                        let x = active_touch.current_x - active_touch.start_x;
                        let y = active_touch.current_y - active_touch.start_y;
                        self.current_pan.x = self.start_pan.x + x.round() as i32;
                        self.current_pan.y = self.start_pan.y + y.round() as i32;
                        return TouchResult::Pan(self.current_pan);
                    }
                    Gesture::Zoom => {
                        let keys: Vec<u64> = self.active_touches.keys().cloned().collect();
                        debug_assert!(keys.len() == 2);
                        let touch0 = &self.active_touches[&keys[0]];
                        let touch1 = &self.active_touches[&keys[1]];
                        let initial_distance = touch0.initial_distance_from_other(touch1);
                        let current_distance = touch0.current_distance_from_other(touch1);
                        self.current_zoom = self.start_zoom * current_distance / initial_distance;
                        return TouchResult::Zoom(self.current_zoom);
                    }
                }
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                self.active_touches.remove(&touch.id).unwrap();
                self.current_gesture = Gesture::None;
            }
        }

        TouchResult::None
    }
}

fn main() {
    let mut app = App {
        touch_state: TouchState::new(),
    };
    boilerplate::main_wrapper(&mut app, None);
}

struct App {
    touch_state: TouchState,
}

impl Example for App {
    // Make this the only example to test all shaders for compile errors.
    const PRECACHE_SHADER_FLAGS: ShaderPrecacheFlags = ShaderPrecacheFlags::FULL_COMPILE;

    fn render(
        &mut self,
        _api: &RenderApi,
        builder: &mut DisplayListBuilder,
        _txn: &mut Transaction,
        _: DeviceUintSize,
        pipeline_id: PipelineId,
        _document_id: DocumentId,
    ) {
        let bounds = LayoutRect::new(LayoutPoint::zero(), builder.content_size());
        let info = LayoutPrimitiveInfo::new(bounds);
        builder.push_stacking_context(
            &info,
            None,
            TransformStyle::Flat,
            MixBlendMode::Normal,
            &[],
            RasterSpace::Screen,
        );

        let asr = builder.define_scroll_frame(None,
                                              (0, 0).to(800, 1000),
                                              (0, 0).to(800, 1000),
                                              vec![],
                                              None,
                                              ScrollSensitivity::Script);

        let rootclip = builder.define_clip_with_parent(ClipId::root_scroll_node(pipeline_id), (0, 0).to(800, 1000), vec![], None);

        // This is the green rect on top
        builder.push_clip_and_scroll_info(ClipAndScrollInfo::new(asr, rootclip));
        builder.push_rect(&LayoutPrimitiveInfo::new((0, 0).to(100, 100)), ColorF::new(0.0, 0.5, 0.0, 1.0));
        builder.pop_clip_id();

        let clip = builder.define_clip_with_parent(asr, (0, 100).to(100, 200), vec![], None);
        let chain = builder.define_clip_chain(None, vec![rootclip, clip]);

        builder.push_clip_and_scroll_info(ClipAndScrollInfo::new(clip, ClipId::ClipChain(chain)));
        let refframe = builder.push_reference_frame(&LayoutPrimitiveInfo::new((0, 100).to(0, 100)), Some(PropertyBinding::Value(LayoutTransform::row_major(
            1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, -50.0, 0.0, 1.0, -1.0, 0.0, -100.0, 0.0, 1.0
        ))), None);
        builder.push_clip_id(refframe);
        builder.push_stacking_context(&LayoutPrimitiveInfo::new((0, 0).to(0, 0)), None, TransformStyle::Flat, MixBlendMode::Normal, &[], RasterSpace::Local(1.0));

        // This is the red and green rect inside the transform
        builder.push_rect(&LayoutPrimitiveInfo::new((0, 0).to(100, 100)), ColorF::new(1.0, 0.0, 0.0, 1.0));
        builder.push_rect(&LayoutPrimitiveInfo::new((0, 100).to(100, 200)), ColorF::new(0.0, 0.5, 0.0, 1.0));

        builder.pop_stacking_context();
        builder.pop_clip_id();
        builder.pop_reference_frame();
        builder.pop_clip_id();

        builder.pop_stacking_context();

        builder.print_display_list();
    }

    fn on_event(&mut self, event: winit::WindowEvent, api: &RenderApi, document_id: DocumentId) -> bool {
        let mut txn = Transaction::new();
        match event {
            winit::WindowEvent::Touch(touch) => match self.touch_state.handle_event(touch) {
                TouchResult::Pan(pan) => {
                    txn.set_pan(pan);
                }
                TouchResult::Zoom(zoom) => {
                    txn.set_pinch_zoom(ZoomFactor::new(zoom));
                }
                TouchResult::None => {}
            },
            _ => (),
        }

        if !txn.is_empty() {
            txn.generate_frame();
            api.send_transaction(document_id, txn);
        }

        false
    }
}
