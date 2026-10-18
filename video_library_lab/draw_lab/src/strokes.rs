pub type Point = (f32, f32);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Brush,
    Eraser,
}

pub struct Stroke {
    pub tool: Tool,
    pub points: Vec<Point>,
}

impl Stroke {
    fn new(tool: Tool) -> Self {
        Self { tool, points: Vec::new() }
    }
}

pub struct StrokeList {
    strokes: Vec<Stroke>,
}

impl StrokeList {
    pub fn new() -> Self {
        Self { strokes: Vec::new() }
    }

    pub fn begin(&mut self, tool: Tool, p: Point) {
        let mut s = Stroke::new(tool);
        s.points.push(p);
        self.strokes.push(s);
    }

    pub fn extend(&mut self, p: Point) {
        if let Some(s) = self.strokes.last_mut() {
            s.points.push(p);
        }
    }

    /// Removes the last point of the last stroke. If that empties the stroke,
    /// the stroke itself is removed. This is the "remove part of the latest tail"
    /// behaviour, not a global undo.
    pub fn undo_tail(&mut self) {
        if let Some(last) = self.strokes.last_mut() {
            let take = (last.points.len() / 4).max(3);
            let new_len = last.points.len().saturating_sub(take);
            last.points.truncate(new_len);
            if last.points.is_empty() {
                self.strokes.pop();
            }
        }
    }

    pub fn undo_stroke(&mut self) {
        self.strokes.pop();
    }

    pub fn clear(&mut self) {
        self.strokes.clear();
    }

    /// Bounding box of all brush strokes in normalized coords.
    /// (min_x, min_y, max_x, max_y). Eraser strokes are ignored.
    pub fn brush_bounds(&self) -> Option<(f32, f32, f32, f32)> {
        let mut min_x = f32::INFINITY;
        let mut min_y = f32::INFINITY;
        let mut max_x = f32::NEG_INFINITY;
        let mut max_y = f32::NEG_INFINITY;
        let mut any = false;
        for s in &self.strokes {
            if s.tool != Tool::Brush {
                continue;
            }
            for (x, y) in &s.points {
                any = true;
                min_x = min_x.min(*x);
                min_y = min_y.min(*y);
                max_x = max_x.max(*x);
                max_y = max_y.max(*y);
            }
        }
        if any { Some((min_x, min_y, max_x, max_y)) } else { None }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Stroke> {
        self.strokes.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.strokes.is_empty()
    }
}

impl Default for StrokeList {
    fn default() -> Self { Self::new() }
}
