use eframe::egui::{self, Color32, Rect};
use std::time::Duration;

const PERIOD: f64 = 6.0;
const SWEEP: f64 = 1.2;

pub fn eligible(value: &str) -> bool {
    crate::data::grade_band(value) == crate::data::GradeBand::High
}

// A soft diagonal reflection, clipped geometrically to the rounded badge.
// The mesh is drawn between the badge background and its text.
fn reflection(rect: Rect, progress: f32) -> egui::Mesh {
    let mut mesh = egui::Mesh::default();
    let radius = 4_f32.min(rect.height() * 0.5).min(rect.width() * 0.5);
    let band = rect.width() * 0.48;
    let travel = rect.width() + rect.height() * 0.45 + band * 2.;
    let center = -band - rect.height() * 0.225 + travel * progress;
    let inset = |y: f32| {
        let distance = y.min(rect.height() - y);
        if distance < radius {
            radius
                - (radius * radius - (radius - distance).powi(2))
                    .max(0.)
                    .sqrt()
        } else {
            0.
        }
    };
    let rows = 16;
    let columns = 20;
    for row in 0..=rows {
        let y = rect.height() * row as f32 / rows as f32;
        let edge = inset(y);
        for column in 0..=columns {
            let x = edge + (rect.width() - 2. * edge) * column as f32 / columns as f32;
            let distance = ((x - center + (y - rect.height() * 0.5) * 0.45) / band).abs();
            let light = if distance < 1. {
                let t = 1. - distance;
                t * t * (3. - 2. * t)
            } else {
                0.
            };
            let fade = (std::f32::consts::PI * progress).sin().max(0.).sqrt();
            mesh.colored_vertex(
                rect.min + egui::vec2(x, y),
                Color32::from_white_alpha((light * fade * 74.) as u8),
            );
        }
    }
    for row in 0..rows {
        for column in 0..columns {
            let a = (row * (columns + 1) + column) as u32;
            let b = a + (columns + 1) as u32;
            mesh.add_triangle(a, a + 1, b);
            mesh.add_triangle(a + 1, b + 1, b);
        }
    }
    mesh
}

pub fn paint(ui: &egui::Ui, shape: egui::layers::ShapeIdx, rect: Rect, offset: u64) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    let (time, focused) = ui.input(|input| (input.time, input.focused));
    let phase = (time + (offset % 1000) as f64 / 1000. * 0.65).rem_euclid(PERIOD);
    if phase < SWEEP {
        ui.painter().set(
            shape,
            egui::Shape::mesh(reflection(rect, (phase / SWEEP) as f32)),
        );
    }
    // Repaint only during a pass; between passes schedule a single wake-up.
    // No timers, network calls or frames are added to the other tabs.
    if focused {
        let delay = if phase < SWEEP {
            Duration::from_millis(33)
        } else {
            Duration::from_secs_f64(PERIOD - phase)
        };
        ui.ctx().request_repaint_after(delay);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_requested_grades_receive_the_effect() {
        for value in ["5", "5+", "6-", "6−", "6", " 6 "] {
            assert!(eligible(value));
        }
        for value in ["0", "2+", "3-", "5-", "5−", "4+", "np", "bdb"] {
            assert!(!eligible(value));
        }
    }
    #[test]
    fn reflection_stays_inside_the_badge_and_has_a_soft_highlight() {
        let rect = Rect::from_min_size(egui::pos2(10., 20.), egui::vec2(48., 34.));
        let mesh = reflection(rect, 0.5);
        assert!(mesh.vertices.iter().all(|v| rect.contains(v.pos)));
        assert!(mesh.vertices.iter().any(|v| v.color.a() > 60));
        assert!(mesh.vertices.iter().any(|v| v.color.a() == 0));
        for vertex in &mesh.vertices {
            let p = vertex.pos - rect.min;
            let closest = egui::pos2(
                p.x.clamp(4., rect.width() - 4.),
                p.y.clamp(4., rect.height() - 4.),
            );
            assert!(
                (egui::pos2(p.x, p.y) - closest).length() <= 4.01,
                "Light must not leak beyond rounded corners"
            );
        }
    }
}
