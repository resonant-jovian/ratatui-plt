//! `DrawingBackend` implementation using tiny-skia for anti-aliased
//! pixel rendering.
//!
//! This backend maps plotters' drawing primitives to tiny-skia's
//! rasterization engine, giving anti-aliased lines, smooth curves,
//! gradient fills, and sub-pixel accuracy.

use plotters_backend::{
    BackendCoord, BackendStyle, BackendTextStyle, DrawingBackend,
    DrawingErrorKind,
};
use tiny_skia::{
    Color, FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap,
    Rect, Stroke, Transform,
};

/// Error type for the tiny-skia drawing backend.
#[derive(Debug)]
pub enum TinySkiaError {
    /// Failed to create a pixmap or path.
    Creation(String),
}

impl std::fmt::Display for TinySkiaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Creation(msg) => write!(f, "tiny-skia error: {msg}"),
        }
    }
}

impl std::error::Error for TinySkiaError {}

/// A plotters `DrawingBackend` backed by a tiny-skia `Pixmap`.
///
/// All drawing operations produce anti-aliased output in an RGBA
/// pixel buffer. The pixmap is shared via `Rc<RefCell<Pixmap>>` so
/// it can be extracted after the `DrawingArea` consumes this backend.
pub struct TinySkiaDrawingBackend {
    pixmap: std::rc::Rc<std::cell::RefCell<Pixmap>>,
}

impl TinySkiaDrawingBackend {
    /// Create a new backend with the given pixel dimensions.
    ///
    /// Returns both the backend and a shared handle to the pixmap.
    /// The handle can be used to extract the pixmap after the
    /// `DrawingArea` has consumed this backend.
    pub fn new(
        width: u32,
        height: u32,
    ) -> Result<
        (Self, std::rc::Rc<std::cell::RefCell<Pixmap>>),
        TinySkiaError,
    > {
        let pixmap = Pixmap::new(width, height).ok_or_else(|| {
            TinySkiaError::Creation(format!(
                "failed to create {width}x{height} pixmap"
            ))
        })?;
        let shared =
            std::rc::Rc::new(std::cell::RefCell::new(pixmap));
        Ok((Self { pixmap: shared.clone() }, shared))
    }

    /// Fill the entire pixmap with a background color.
    pub fn fill_background(&mut self, r: u8, g: u8, b: u8) {
        self.pixmap
            .borrow_mut()
            .fill(Color::from_rgba8(r, g, b, 255));
    }

    fn make_paint_rgba(
        r: u8,
        g: u8,
        b: u8,
        a: f64,
    ) -> Paint<'static> {
        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };
        paint.set_color_rgba8(r, g, b, (a * 255.0) as u8);
        paint
    }

    fn paint_from_style(style: &impl BackendStyle) -> Paint<'static> {
        let c = style.color();
        let (r, g, b) = c.rgb;
        Self::make_paint_rgba(r, g, b, c.alpha)
    }

    fn make_stroke(width: u32) -> Stroke {
        Stroke {
            width: width.max(1) as f32,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        }
    }
}

impl DrawingBackend for TinySkiaDrawingBackend {
    type ErrorType = TinySkiaError;

    fn get_size(&self) -> (u32, u32) {
        let pm = self.pixmap.borrow();
        (pm.width(), pm.height())
    }

    fn ensure_prepared(
        &mut self,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        Ok(())
    }

    fn present(
        &mut self,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        Ok(())
    }

    fn draw_pixel(
        &mut self,
        point: BackendCoord,
        color: plotters_backend::BackendColor,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let (x, y) = point;
        let mut pm = self.pixmap.borrow_mut();
        if x >= 0
            && y >= 0
            && (x as u32) < pm.width()
            && (y as u32) < pm.height()
        {
            let (r, g, b) = color.rgb;
            if let Some(rect) = Rect::from_xywh(
                x as f32,
                y as f32,
                1.0,
                1.0,
            ) {
                let paint = Self::make_paint_rgba(
                    r,
                    g,
                    b,
                    color.alpha,
                );
                pm.fill_rect(
                    rect,
                    &paint,
                    Transform::identity(),
                    None,
                );
            }
        }
        Ok(())
    }

    fn draw_line<S: BackendStyle>(
        &mut self,
        from: BackendCoord,
        to: BackendCoord,
        style: &S,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let mut pb = PathBuilder::new();
        pb.move_to(from.0 as f32, from.1 as f32);
        pb.line_to(to.0 as f32, to.1 as f32);
        if let Some(path) = pb.finish() {
            let paint = Self::paint_from_style(style);
            let stroke = Self::make_stroke(style.stroke_width());
            self.pixmap.borrow_mut().stroke_path(
                &path,
                &paint,
                &stroke,
                Transform::identity(),
                None,
            );
        }
        Ok(())
    }

    fn draw_rect<S: BackendStyle>(
        &mut self,
        upper_left: BackendCoord,
        bottom_right: BackendCoord,
        style: &S,
        fill: bool,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let x = upper_left.0.min(bottom_right.0) as f32;
        let y = upper_left.1.min(bottom_right.1) as f32;
        let w = (bottom_right.0 - upper_left.0).unsigned_abs() as f32;
        let h = (bottom_right.1 - upper_left.1).unsigned_abs() as f32;

        if let Some(rect) = Rect::from_xywh(x, y, w.max(1.0), h.max(1.0))
        {
            let paint = Self::paint_from_style(style);
            if fill {
                self.pixmap.borrow_mut().fill_rect(
                    rect,
                    &paint,
                    Transform::identity(),
                    None,
                );
            } else {
                let mut pb = PathBuilder::new();
                pb.move_to(x, y);
                pb.line_to(x + w, y);
                pb.line_to(x + w, y + h);
                pb.line_to(x, y + h);
                pb.close();
                if let Some(path) = pb.finish() {
                    let stroke = Self::make_stroke(style.stroke_width());
                    self.pixmap.borrow_mut().stroke_path(
                        &path,
                        &paint,
                        &stroke,
                        Transform::identity(),
                        None,
                    );
                }
            }
        }
        Ok(())
    }

    fn draw_path<S: BackendStyle, I: IntoIterator<Item = BackendCoord>>(
        &mut self,
        path: I,
        style: &S,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let mut pb = PathBuilder::new();
        let mut started = false;
        for (x, y) in path {
            if !started {
                pb.move_to(x as f32, y as f32);
                started = true;
            } else {
                pb.line_to(x as f32, y as f32);
            }
        }
        if started {
            if let Some(path) = pb.finish() {
                let paint = Self::paint_from_style(style);
                let stroke = Self::make_stroke(style.stroke_width());
                self.pixmap.borrow_mut().stroke_path(
                    &path,
                    &paint,
                    &stroke,
                    Transform::identity(),
                    None,
                );
            }
        }
        Ok(())
    }

    fn draw_circle<S: BackendStyle>(
        &mut self,
        center: BackendCoord,
        radius: u32,
        style: &S,
        fill: bool,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let cx = center.0 as f32;
        let cy = center.1 as f32;
        let r = radius as f32;

        // Approximate circle with 4 cubic beziers.
        let k = 0.552_284_8; // magic constant for cubic circle approx
        let mut pb = PathBuilder::new();
        pb.move_to(cx + r, cy);
        pb.cubic_to(cx + r, cy + r * k, cx + r * k, cy + r, cx, cy + r);
        pb.cubic_to(cx - r * k, cy + r, cx - r, cy + r * k, cx - r, cy);
        pb.cubic_to(cx - r, cy - r * k, cx - r * k, cy - r, cx, cy - r);
        pb.cubic_to(cx + r * k, cy - r, cx + r, cy - r * k, cx + r, cy);
        pb.close();

        if let Some(path) = pb.finish() {
            let paint = Self::paint_from_style(style);
            if fill {
                self.pixmap.borrow_mut().fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            } else {
                let stroke = Self::make_stroke(style.stroke_width());
                self.pixmap.borrow_mut().stroke_path(
                    &path,
                    &paint,
                    &stroke,
                    Transform::identity(),
                    None,
                );
            }
        }
        Ok(())
    }

    fn fill_polygon<S: BackendStyle, I: IntoIterator<Item = BackendCoord>>(
        &mut self,
        vert: I,
        style: &S,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let mut pb = PathBuilder::new();
        let mut started = false;
        for (x, y) in vert {
            if !started {
                pb.move_to(x as f32, y as f32);
                started = true;
            } else {
                pb.line_to(x as f32, y as f32);
            }
        }
        if started {
            pb.close();
            if let Some(path) = pb.finish() {
                let paint = Self::paint_from_style(style);
                self.pixmap.borrow_mut().fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }
        }
        Ok(())
    }

    fn draw_text<TStyle: BackendTextStyle>(
        &mut self,
        text: &str,
        style: &TStyle,
        pos: BackendCoord,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        // Simple bitmap text rendering — each character is a filled
        // rectangle with approximate glyph width. For production
        // quality, integrate cosmic-text or fontdue.
        let c = style.color();
        let (r, g, b) = c.rgb;
        let font_size = style.size() as f32;
        let char_w = font_size * 0.6;
        let mut x = pos.0 as f32;
        let y = pos.1 as f32;

        let mut paint = Paint {
            anti_alias: true,
            ..Paint::default()
        };
        paint.set_color_rgba8(r, g, b, 255);

        for _ch in text.chars() {
            // Placeholder: draw small filled rect per character.
            // This gives approximate text positioning for layout
            // purposes. Full glyph rendering is a future enhancement.
            if let Some(rect) = Rect::from_xywh(
                x,
                y,
                char_w * 0.8,
                font_size * 0.8,
            ) {
                self.pixmap.borrow_mut().fill_rect(
                    rect,
                    &paint,
                    Transform::identity(),
                    None,
                );
            }
            x += char_w;
        }
        Ok(())
    }

    fn estimate_text_size<TStyle: BackendTextStyle>(
        &self,
        text: &str,
        style: &TStyle,
    ) -> Result<(u32, u32), DrawingErrorKind<Self::ErrorType>> {
        let font_size = style.size();
        let char_w = (font_size * 0.6) as u32;
        let width = char_w * text.len() as u32;
        let height = font_size as u32;
        Ok((width, height))
    }
}
