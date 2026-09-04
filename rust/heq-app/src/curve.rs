use eframe::egui::{
    self, Align2, FontId, Mesh, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, Vec2,
};

use heq_core::dsp::Biquad;
use heq_core::format;
use heq_core::model::{BandId, ChannelTarget, EqBand, EqModel, FilterKind};
use heq_core::ui::geometry::{Plot, DEFAULT_Q, HANDLE_HIT_RADIUS, HANDLE_RADIUS};
use heq_core::ui::{geometry, palette};

use crate::theme::col;

const SLOPES: [i32; 4] = [12, 24, 36, 48];
const LABEL_SIZE: f32 = 9.5;

#[derive(Clone, Copy, PartialEq)]
enum DragMode {
    FreqGain,
    Q,
}

struct Drag {
    band: BandId,
    mode: DragMode,
    start: Pos2,
    freq: f64,
    gain: f64,
    q: f64,
}

pub struct CurveState {
    pub selected: Option<BandId>,
    pub db_range: f64,
    pub place_on: ChannelTarget,
    hovered: Option<BandId>,
    drag: Option<Drag>,
    just_added: Option<BandId>,
}

impl Default for CurveState {
    fn default() -> Self {
        CurveState {
            selected: None,
            db_range: 18.0,
            place_on: ChannelTarget::Both,
            hovered: None,
            drag: None,
            just_added: None,
        }
    }
}

impl CurveState {
    pub fn new(db_range: f64) -> Self {
        CurveState {
            db_range,
            ..Default::default()
        }
    }
}

pub fn show(ui: &mut Ui, model: &mut EqModel, overlay: Option<&EqModel>, state: &mut CurveState) {
    let size = ui.available_size();
    let (rect, response) = ui.allocate_exact_size(size, Sense::click_and_drag());
    let plot = Plot::new(rect.width() as f64, rect.height() as f64, state.db_range);

    let pointer = response
        .hover_pos()
        .or_else(|| ui.input(|i| i.pointer.interact_pos()))
        .map(|p| p - rect.min.to_vec2());

    input(ui, model, state, &plot, &response, pointer);

    let ghost_on = pointer.is_some_and(|p| in_plot(&plot, p))
        && state.drag.is_none()
        && hit_test(model, &plot, pointer.unwrap()).is_none();
    let ghost = ui
        .ctx()
        .animate_bool_with_time(response.id.with("ghost"), ghost_on, 0.15);

    paint(ui, model, overlay, state, &plot, rect, pointer, ghost);
}

// ==== painting ====

fn paint(
    ui: &Ui,
    model: &EqModel,
    overlay: Option<&EqModel>,
    state: &CurveState,
    plot: &Plot,
    rect: Rect,
    pointer: Option<Pos2>,
    ghost: f32,
) {
    let p = ui.painter_at(rect);
    let at = |x: f64, y: f64| rect.min + Vec2::new(x as f32, y as f32);

    p.rect_filled(rect, 0.0, col(palette::BG_TOP));
    grid(&p, plot, at);

    let freqs = sample_frequencies(plot);
    let sr = model.sample_rate();

    // one section list per band per frame; response_db would rebuild them per pixel
    let baked: Vec<(usize, &EqBand, Vec<Biquad>)> = model
        .bands()
        .iter()
        .enumerate()
        .map(|(i, b)| (i, b, b.sections(sr)))
        .collect();

    band_curves(&p, &baked, state, plot, &freqs, sr, at);
    total_curve(&p, model, overlay, plot, &freqs, at);
    handles(&p, model, state, plot, at);

    if let Some(pos) = pointer.filter(|q| in_plot(plot, *q) && state.drag.is_none()) {
        let x = snap(pos.x as f64);
        p.line_segment(
            [at(x, 0.0), at(x, plot.plot_height())],
            Stroke::new(1.0, col(palette::CROSSHAIR)),
        );
    }

    if ghost > 0.01 {
        if let Some(pos) = pointer {
            placement_preview(ui, &p, plot, &freqs, sr, pos, ghost, at);
        }
    }
}

fn grid(p: &egui::Painter, plot: &Plot, at: impl Fn(f64, f64) -> Pos2) {
    let (pw, ph) = (plot.plot_width(), plot.plot_height());
    let label_font = FontId::proportional(LABEL_SIZE);

    for (freq, label) in geometry::frequency_ticks() {
        let x = snap(plot.freq_to_x(freq));
        if x < 0.0 || x > pw {
            continue;
        }

        let stroke = Stroke::new(
            1.0,
            col(if label.is_some() {
                palette::GRID_MAJOR
            } else {
                palette::GRID_MINOR
            }),
        );
        p.line_segment([at(x, 0.0), at(x, ph)], stroke);

        if let Some(label) = label {
            p.text(
                at(x, ph + 3.0),
                Align2::CENTER_TOP,
                label,
                label_font.clone(),
                col(palette::LABEL),
            );
        }
    }

    for db in plot.db_lines() {
        let y = snap(plot.db_to_y(db));
        let zero = db.abs() < 0.001;
        p.line_segment(
            [at(0.0, y), at(pw, y)],
            Stroke::new(
                1.0,
                col(if zero {
                    palette::ZERO_LINE
                } else {
                    palette::GRID_MINOR
                }),
            ),
        );

        let label = if zero {
            "0".to_string()
        } else {
            format::gain_label(db)
        };
        p.text(
            at(pw + 5.0, y),
            Align2::LEFT_CENTER,
            label,
            label_font.clone(),
            col(palette::LABEL),
        );
    }
}

fn sample_frequencies(plot: &Plot) -> Vec<f64> {
    let columns = (plot.plot_width().ceil() as usize).max(2);
    (0..columns)
        .map(|i| plot.x_to_freq(i as f64 * plot.plot_width() / (columns - 1) as f64))
        .collect()
}

fn trace(
    plot: &Plot,
    freqs: &[f64],
    at: &impl Fn(f64, f64) -> Pos2,
    mut db_at: impl FnMut(f64) -> f64,
) -> Vec<Pos2> {
    let last = (freqs.len() - 1) as f64;
    freqs
        .iter()
        .enumerate()
        .map(|(i, f)| {
            at(
                i as f64 * plot.plot_width() / last,
                plot.db_to_y(db_at(*f)),
            )
        })
        .collect()
}

fn sections_db(sections: &[Biquad], f: f64, sr: f64) -> f64 {
    let w = 2.0 * std::f64::consts::PI * f / sr;
    sections.iter().map(|s| s.gain_db(w)).sum()
}

fn band_curves(
    p: &egui::Painter,
    baked: &[(usize, &EqBand, Vec<Biquad>)],
    state: &CurveState,
    plot: &Plot,
    freqs: &[f64],
    sr: f64,
    at: impl Fn(f64, f64) -> Pos2,
) {
    for (i, band, sections) in baked {
        if !band.enabled {
            continue;
        }

        let selected = state.selected == Some(band.id);
        let color = palette::with_alpha(
            palette::band_color(*i),
            if selected { 230 } else { 120 },
        );
        let stroke = Stroke::new(if selected { 1.6 } else { 1.1 }, col(color));

        let pts = trace(plot, freqs, &at, |f| sections_db(sections, f, sr));
        p.add(Shape::line(pts, stroke));
    }
}

fn total_curve(
    p: &egui::Painter,
    model: &EqModel,
    overlay: Option<&EqModel>,
    plot: &Plot,
    freqs: &[f64],
    at: impl Fn(f64, f64) -> Pos2,
) {
    let zero_y = plot.db_to_y(0.0);
    let split = model.has_per_channel_bands()
        || overlay.is_some_and(|o| o.has_per_channel_bands());

    let draw = |channel: ChannelTarget, color: palette::Rgba, width: f32, fill: bool| {
        let pts = trace(plot, freqs, &at, |f| {
            model.response_db(f, channel) + overlay.map_or(0.0, |o| o.response_db(f, channel))
        });

        if fill {
            p.add(area_under(&pts, at(0.0, zero_y).y));
        }
        p.add(Shape::line(pts, Stroke::new(width, col(color))));
    };

    if split {
        draw(ChannelTarget::Both, palette::SHARED, 1.0, true);
        draw(ChannelTarget::Left, palette::LEFT, 1.4, false);
        draw(ChannelTarget::Right, palette::RIGHT, 1.4, false);
    } else {
        draw(ChannelTarget::Both, palette::TOTAL, 1.8, true);
    }
}

// egui only fills convex paths, so the area is a quad strip down to the 0 dB line
fn area_under(pts: &[Pos2], zero_y: f32) -> Shape {
    let mut mesh = Mesh::default();
    let top = col(palette::AREA_TOP);
    let bottom = col(palette::AREA_BOTTOM);

    for pair in pts.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let base = mesh.vertices.len() as u32;

        mesh.colored_vertex(a, top);
        mesh.colored_vertex(b, top);
        mesh.colored_vertex(Pos2::new(b.x, zero_y), bottom);
        mesh.colored_vertex(Pos2::new(a.x, zero_y), bottom);

        mesh.add_triangle(base, base + 1, base + 2);
        mesh.add_triangle(base, base + 2, base + 3);
    }

    Shape::mesh(mesh)
}

fn handles(
    p: &egui::Painter,
    model: &EqModel,
    state: &CurveState,
    plot: &Plot,
    at: impl Fn(f64, f64) -> Pos2,
) {
    for (i, band) in model.bands().iter().enumerate() {
        let x = plot.freq_to_x(band.freq);
        if x < -20.0 || x > plot.plot_width() + 20.0 {
            continue;
        }

        let color = palette::band_color(i);
        let selected = state.selected == Some(band.id);
        let hovered = state.hovered == Some(band.id);

        let y = handle_y(plot, band);
        let r = (HANDLE_RADIUS
            * if selected {
                1.35
            } else if hovered {
                1.15
            } else {
                1.0
            }) as f32;

        if selected {
            p.circle_filled(
                at(x, y),
                r * 2.4,
                col(palette::with_alpha(color, 0x35)),
            );
            p.line_segment(
                [at(x, 0.0), at(x, plot.plot_height())],
                Stroke::new(1.0, col(palette::with_alpha(color, 0x50))),
            );
        }

        let fill = if band.enabled {
            col(palette::with_alpha(color, if selected { 0xFF } else { 0xD0 }))
        } else {
            col(palette::BYPASSED_FILL)
        };
        let edge = Stroke::new(
            1.5,
            col(if band.enabled {
                palette::HANDLE_EDGE
            } else {
                palette::with_alpha(color, 0x90)
            }),
        );

        p.circle(at(x, y), r, fill, edge);
        p.text(
            at(x, y),
            Align2::CENTER_CENTER,
            (i + 1).to_string(),
            FontId::proportional(if selected { 9.5 } else { 8.5 }),
            if band.enabled {
                col(palette::HANDLE_TEXT)
            } else {
                col(palette::LABEL)
            },
        );
    }
}

fn placement_preview(
    ui: &Ui,
    p: &egui::Painter,
    plot: &Plot,
    freqs: &[f64],
    sr: f64,
    pos: Pos2,
    ghost: f32,
    at: impl Fn(f64, f64) -> Pos2,
) {
    let shift = ui.input(|i| i.modifiers.shift);
    let kind = kind_for(plot.x_to_freq(pos.x as f64), shift);

    let mut preview = EqBand::new(BandId(0));
    preview.kind = kind;
    preview.freq = plot.x_to_freq(pos.x as f64);
    preview.gain_db = (plot.clamp_to_range(plot.y_to_db(pos.y as f64)) * 10.0).round() / 10.0;
    preview.q = DEFAULT_Q;
    preview.clamp();

    let sections = preview.sections(sr);
    let alpha = |a: f32| (a * ghost) as u8;

    let pts = trace(plot, freqs, &at, |f| sections_db(&sections, f, sr));
    p.add(Shape::dashed_line(
        &pts,
        Stroke::new(1.3, col(palette::with_alpha(palette::ACCENT, alpha(0x99 as f32)))),
        4.0,
        3.0,
    ));

    let hx = pos.x as f64;
    let hy = handle_y(plot, &preview);
    let ring = Stroke::new(
        1.4,
        col(palette::with_alpha(palette::ACCENT, alpha(0xCC as f32))),
    );

    p.circle(
        at(hx, hy),
        HANDLE_RADIUS as f32,
        col(palette::with_alpha(palette::ACCENT, alpha(0x30 as f32))),
        ring,
    );
    p.line_segment([at(hx - 2.5, hy), at(hx + 2.5, hy)], ring);
    p.line_segment([at(hx, hy - 2.5), at(hx, hy + 2.5)], ring);

    // the badge says what a click will make, so the interface needs no instructions
    let badge = at(hx + 15.0, (hy - 26.0).clamp(4.0, plot.plot_height() - 20.0));
    p.text(
        badge,
        Align2::LEFT_TOP,
        kind.display_name(),
        FontId::proportional(10.0),
        col(palette::with_alpha(palette::ACCENT, alpha(0xFF as f32))),
    );
}

// ==== input ====

fn input(
    ui: &Ui,
    model: &mut EqModel,
    state: &mut CurveState,
    plot: &Plot,
    response: &Response,
    pointer: Option<Pos2>,
) {
    let (shift, ctrl, alt) = ui.input(|i| {
        (
            i.modifiers.shift,
            i.modifiers.command || i.modifiers.ctrl,
            i.modifiers.alt,
        )
    });

    state.hovered = pointer.and_then(|p| hit_test(model, plot, p));

    if response.double_clicked() {
        if let Some(p) = pointer {
            if let Some(hit) = hit_test(model, plot, p) {
                if Some(hit) != state.just_added {
                    remove(model, state, hit);
                }
            }
        }
        state.just_added = None;
        state.drag = None;
        return;
    }

    if response.drag_started() || response.clicked() {
        if let Some(p) = pointer {
            let hit = hit_test(model, plot, p);

            let band_id = match hit {
                Some(id) => {
                    state.just_added = None;
                    if alt {
                        model.edit(id, |b| b.enabled = !b.enabled);
                        state.selected = Some(id);
                        return;
                    }
                    id
                }
                None => {
                    let id = add_band_at(model, state, plot, p, shift);
                    state.just_added = Some(id);
                    id
                }
            };

            state.selected = Some(band_id);
            if let Some(band) = model.band(band_id) {
                state.drag = Some(Drag {
                    band: band_id,
                    mode: if ctrl { DragMode::Q } else { DragMode::FreqGain },
                    start: p,
                    freq: band.freq,
                    gain: band.gain_db,
                    q: band.q,
                });
            }
        }
    }

    if response.dragged() {
        if let (Some(p), Some(drag)) = (pointer, state.drag.as_ref()) {
            let fine = if shift { 0.22 } else { 1.0 };
            let dy = (p.y - drag.start.y) as f64 * fine;

            match drag.mode {
                DragMode::Q => {
                    let q = drag.q * 2f64.powf(dy / 60.0);
                    model.edit(drag.band, |b| b.q = q);
                }
                DragMode::FreqGain => {
                    let dx = (p.x - drag.start.x) as f64 * fine;
                    let freq = plot.x_to_freq(plot.freq_to_x(drag.freq) + dx);
                    let gain = plot.clamp_to_range(
                        drag.gain - dy / (plot.plot_height() * 0.5) * plot.db_range,
                    );
                    model.edit(drag.band, |b| {
                        b.freq = freq;
                        if b.kind.uses_gain() {
                            b.gain_db = gain;
                        }
                    });
                }
            }
        }
    }

    if response.drag_stopped() {
        state.drag = None;
    }

    if response.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            let steps = (scroll / 50.0).round() as i32;
            if steps != 0 {
                let target = pointer.and_then(|p| hit_test(model, plot, p)).or(state.selected);
                if let Some(id) = target {
                    let uses_slope = model.band(id).is_some_and(|b| b.kind.uses_slope());
                    model.edit(id, |b| {
                        if uses_slope {
                            let i = SLOPES.iter().position(|s| *s == b.slope_db_per_oct).unwrap_or(0);
                            let next = (i as i32 + steps).clamp(0, SLOPES.len() as i32 - 1);
                            b.slope_db_per_oct = SLOPES[next as usize];
                        } else {
                            b.q *= (if shift { 1.03 } else { 1.12f64 }).powi(steps);
                        }
                    });
                    state.selected = Some(id);
                }
            }
        }
    }

    keyboard(ui, model, state, plot, shift);
    band_menu(model, state, response);
}

fn keyboard(ui: &Ui, model: &mut EqModel, state: &mut CurveState, plot: &Plot, fine: bool) {
    let Some(id) = state.selected else { return };

    let keys = ui.input(|i| {
        [
            i.key_pressed(egui::Key::Escape),
            i.key_pressed(egui::Key::Delete) || i.key_pressed(egui::Key::Backspace),
            i.key_pressed(egui::Key::ArrowLeft),
            i.key_pressed(egui::Key::ArrowRight),
            i.key_pressed(egui::Key::ArrowUp),
            i.key_pressed(egui::Key::ArrowDown),
        ]
    });

    if keys[0] {
        state.selected = None;
        return;
    }
    if keys[1] {
        remove(model, state, id);
        return;
    }

    let scale = if keys[2] {
        Some(if fine { 0.999 } else { 0.98 })
    } else if keys[3] {
        Some(if fine { 1.001 } else { 1.02 })
    } else {
        None
    };

    if let Some(scale) = scale {
        model.edit(id, |b| b.freq *= scale);
    }

    let nudge = if keys[4] {
        Some(if fine { 0.1 } else { 0.5 })
    } else if keys[5] {
        Some(if fine { -0.1 } else { -0.5 })
    } else {
        None
    };

    if let Some(db) = nudge {
        let range = plot.db_range;
        model.edit(id, |b| {
            if b.kind.uses_gain() {
                b.gain_db = (b.gain_db + db).clamp(-range, range);
            }
        });
    }
}

fn band_menu(model: &mut EqModel, state: &mut CurveState, response: &Response) {
    let Some(id) = state.hovered.or(state.selected) else {
        return;
    };
    let Some(band) = model.band(id).copied() else {
        return;
    };

    response.context_menu(|ui| {
        for kind in FilterKind::ALL {
            if ui
                .selectable_label(band.kind == kind, kind.display_name())
                .clicked()
            {
                model.edit(id, |b| b.kind = kind);
                ui.close();
            }
        }

        ui.separator();

        for ear in ChannelTarget::ALL {
            if ui
                .selectable_label(band.channel == ear, ear.ear_name())
                .clicked()
            {
                model.edit(id, |b| b.channel = ear);
                ui.close();
            }
        }

        ui.separator();

        let bypass = if band.enabled {
            "Bypass band"
        } else {
            "Enable band"
        };
        if ui.button(bypass).clicked() {
            model.edit(id, |b| b.enabled = !b.enabled);
            ui.close();
        }
        if ui.button("Delete band").clicked() {
            remove(model, state, id);
            ui.close();
        }
    });
}

// ==== helpers ====

fn shelf_split() -> f64 {
    (geometry::FREQ_MIN * geometry::FREQ_MAX).sqrt()
}

pub fn kind_for(freq: f64, shift: bool) -> FilterKind {
    if shift {
        return if freq < shelf_split() {
            FilterKind::LowShelf
        } else {
            FilterKind::HighShelf
        };
    }

    if freq < 45.0 {
        FilterKind::LowShelf
    } else if freq > 12000.0 {
        FilterKind::HighShelf
    } else {
        FilterKind::Bell
    }
}

fn add_band_at(
    model: &mut EqModel,
    state: &CurveState,
    plot: &Plot,
    p: Pos2,
    shift: bool,
) -> BandId {
    let freq = plot.x_to_freq(p.x as f64);
    let gain = (plot.clamp_to_range(plot.y_to_db(p.y as f64)) * 10.0).round() / 10.0;

    let id = model.add_band(kind_for(freq, shift), freq, gain, DEFAULT_Q);
    let channel = state.place_on;
    model.edit(id, |b| b.channel = channel);
    id
}

fn remove(model: &mut EqModel, state: &mut CurveState, id: BandId) {
    model.remove_band(id);
    if state.selected == Some(id) {
        state.selected = None;
    }
    if state.just_added == Some(id) {
        state.just_added = None;
    }
}

fn hit_test(model: &EqModel, plot: &Plot, p: Pos2) -> Option<BandId> {
    let mut best = None;
    let mut best_dist = HANDLE_HIT_RADIUS;

    for band in model.bands().iter().rev() {
        let dx = p.x as f64 - plot.freq_to_x(band.freq);
        let dy = p.y as f64 - handle_y(plot, band);
        let d = (dx * dx + dy * dy).sqrt();

        if d <= best_dist {
            best_dist = d;
            best = Some(band.id);
        }
    }
    best
}

fn handle_y(plot: &Plot, band: &EqBand) -> f64 {
    plot.db_to_y(if band.kind.uses_gain() {
        band.gain_db
    } else {
        0.0
    })
}

fn in_plot(plot: &Plot, p: Pos2) -> bool {
    p.x >= 0.0
        && p.x as f64 <= plot.plot_width()
        && p.y >= 0.0
        && p.y as f64 <= plot.plot_height()
}

fn snap(v: f64) -> f64 {
    v.round() + 0.5
}

