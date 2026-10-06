use std::f32::consts::TAU;

use eframe::egui::{self, pos2, vec2, Color32, Painter, Pos2, Rect, Stroke};


#[derive(Clone, Copy)]
pub enum Icon {
    Plus,
    Close,
    Maximize,
    Restore,
    SplitRight,
    SplitDown,
    Gear,
    Back,
    Folder,
    Server,
    Dots,
    Plug,
    Keyboard,
    Download,
    Reset,
    Check,
    Files,
}

pub fn draw(p: &Painter, c: Pos2, icon: Icon, col: Color32) {
    let s = Stroke::new(1.3, col);
    let b = Rect::from_center_size(c, vec2(12.0, 11.0));
    let line = |a: egui::Vec2, z: egui::Vec2| p.line_segment([c + a, c + z], s);
    match icon {
        Icon::Plus => {
            line(vec2(-6.0, 0.0), vec2(6.0, 0.0));
            line(vec2(0.0, -6.0), vec2(0.0, 6.0));
        }
        Icon::Close => {
            line(vec2(-4.5, -4.5), vec2(4.5, 4.5));
            line(vec2(-4.5, 4.5), vec2(4.5, -4.5));
        }
        Icon::Maximize => {
            line(vec2(1.0, -5.0), vec2(5.0, -5.0));
            line(vec2(5.0, -5.0), vec2(5.0, -1.0));
            line(vec2(-1.0, 5.0), vec2(-5.0, 5.0));
            line(vec2(-5.0, 5.0), vec2(-5.0, 1.0));
        }
        Icon::Restore => {
            line(vec2(1.0, -5.0), vec2(1.0, -1.0));
            line(vec2(1.0, -1.0), vec2(5.0, -1.0));
            line(vec2(-1.0, 5.0), vec2(-1.0, 1.0));
            line(vec2(-1.0, 1.0), vec2(-5.0, 1.0));
        }
        Icon::SplitRight => {
            p.rect_stroke(b, 2.0, s, egui::StrokeKind::Middle);
            p.line_segment([pos2(c.x, b.top()), pos2(c.x, b.bottom())], s);
        }
        Icon::SplitDown => {
            p.rect_stroke(b, 2.0, s, egui::StrokeKind::Middle);
            p.line_segment([pos2(b.left(), c.y), pos2(b.right(), c.y)], s);
        }
        Icon::Gear => {
            p.circle_stroke(c, 4.2, s);
            p.circle_stroke(c, 1.6, s);
            for k in 0..8 {
                let d = egui::Vec2::angled(k as f32 * TAU / 8.0);
                p.line_segment([c + d * 4.2, c + d * 6.6], Stroke::new(2.0, col));
            }
        }
        Icon::Back => {
            line(vec2(-6.0, 0.0), vec2(6.0, 0.0));
            line(vec2(-6.0, 0.0), vec2(-2.0, -4.0));
            line(vec2(-6.0, 0.0), vec2(-2.0, 4.0));
        }
        Icon::Folder => {
            let pts = vec![
                c + vec2(-9.0, -6.0),
                c + vec2(-3.0, -6.0),
                c + vec2(-1.0, -3.5),
                c + vec2(9.0, -3.5),
                c + vec2(9.0, 7.0),
                c + vec2(-9.0, 7.0),
            ];
            p.add(egui::Shape::closed_line(pts, Stroke::new(1.5, col)));
        }
        Icon::Dots => {
            for dx in [-5.0, 0.0, 5.0] {
                p.circle_filled(c + vec2(dx, 0.0), 1.4, col);
            }
        }
        Icon::Plug => {
            p.rect_stroke(Rect::from_center_size(c + vec2(0.0, -0.5), vec2(10.0, 7.0)), 2.0, s, egui::StrokeKind::Middle);
            line(vec2(-2.5, -4.0), vec2(-2.5, -7.0));
            line(vec2(2.5, -4.0), vec2(2.5, -7.0));
            line(vec2(0.0, 3.0), vec2(0.0, 7.0));
        }
        Icon::Keyboard => {
            p.rect_stroke(Rect::from_center_size(c, vec2(15.0, 10.0)), 2.0, s, egui::StrokeKind::Middle);
            for dx in [-4.0, 0.0, 4.0] {
                p.circle_filled(c + vec2(dx, -1.8), 0.9, col);
            }
            line(vec2(-3.5, 2.2), vec2(3.5, 2.2));
        }
        Icon::Download => {
            line(vec2(0.0, -6.0), vec2(0.0, 3.0));
            line(vec2(-3.5, -0.5), vec2(0.0, 3.0));
            line(vec2(3.5, -0.5), vec2(0.0, 3.0));
            line(vec2(-6.0, 6.0), vec2(6.0, 6.0));
        }
        Icon::Reset => {
            let pts: Vec<Pos2> = (0..=20).map(|k| c + egui::Vec2::angled(-2.2 + k as f32 / 20.0 * 4.9) * 5.0).collect();
            let tip = pts[0];
            p.add(egui::Shape::line(pts, s));
            p.line_segment([tip, tip + vec2(0.5, -3.2)], s);
            p.line_segment([tip, tip + vec2(3.2, 0.3)], s);
        }
        Icon::Check => {
            line(vec2(-4.5, 0.0), vec2(-1.5, 3.0));
            line(vec2(-1.5, 3.0), vec2(4.5, -3.5));
        }
        Icon::Files => {
            p.rect_stroke(Rect::from_center_size(c, vec2(13.0, 12.0)), 2.0, s, egui::StrokeKind::Middle);
            line(vec2(1.5, -6.0), vec2(1.5, 6.0));
        }
        Icon::Server => {
            for dy in [-4.5, 4.5] {
                let r = Rect::from_center_size(c + vec2(0.0, dy), vec2(18.0, 7.0));
                p.rect_stroke(r, 2.0, Stroke::new(1.5, col), egui::StrokeKind::Middle);
                p.circle_filled(r.left_center() + vec2(4.0, 0.0), 1.2, col);
            }
        }
    }
}

