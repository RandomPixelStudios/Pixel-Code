use serde::{Deserialize, Serialize};

pub type PaneId = u64;

pub const MAX_COLS: usize = 4;
pub const MAX_ROWS: usize = 2;
pub const MAX_PANES: usize = MAX_COLS * MAX_ROWS;

/// Raster aus Terminal-Panes: höchstens 2 Zeilen mit je höchstens 4 Spalten.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Grid {
    pub rows: Vec<Vec<PaneId>>,
    /// Relative Spaltenbreiten pro Zeile.
    pub widths: Vec<Vec<f32>>,
    /// Anteil der oberen Zeile an der Höhe (nur bei 2 Zeilen).
    pub row_ratio: f32,
}

impl Default for Grid {
    fn default() -> Self {
        Self { rows: Vec::new(), widths: Vec::new(), row_ratio: 0.5 }
    }
}

impl Grid {
    pub fn panes(&self) -> Vec<PaneId> {
        self.rows.iter().flatten().copied().collect()
    }

    pub fn len(&self) -> usize {
        self.rows.iter().map(Vec::len).sum()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_full(&self) -> bool {
        self.len() >= MAX_PANES
    }

    pub fn contains(&self, id: PaneId) -> bool {
        self.pos(id).is_some()
    }

    fn pos(&self, id: PaneId) -> Option<(usize, usize)> {
        self.rows.iter().enumerate().find_map(|(r, row)| row.iter().position(|p| *p == id).map(|c| (r, c)))
    }

    fn row_len(&self, r: usize) -> usize {
        self.rows.get(r).map_or(0, Vec::len)
    }

    fn insert(&mut self, r: usize, c: usize, id: PaneId) {
        while self.rows.len() <= r {
            self.rows.push(Vec::new());
        }
        self.rows[r].insert(c, id);
        self.reset();
    }

    /// Setzt alle Größen auf gleichmäßig zurück und stellt Konsistenz her.
    pub fn reset(&mut self) {
        self.rows.retain(|r| !r.is_empty());
        self.widths = self.rows.iter().map(|r| vec![1.0; r.len()]).collect();
        self.row_ratio = 0.5;
    }

    pub fn normalize(&mut self) {
        let ok = self.widths.len() == self.rows.len()
            && self.widths.iter().zip(&self.rows).all(|(w, r)| w.len() == r.len());
        if !ok {
            self.reset();
        }
    }

    /// Hängt eine Pane an: erst oben bis 4 Spalten, danach unten.
    pub fn push(&mut self, id: PaneId) -> bool {
        for r in 0..MAX_ROWS {
            let n = self.row_len(r);
            if n < MAX_COLS {
                self.insert(r, n, id);
                return true;
            }
        }
        false
    }

    pub fn split_right(&mut self, target: PaneId, id: PaneId) -> bool {
        let Some((r, c)) = self.pos(target).filter(|_| !self.is_full()) else { return false };
        if self.row_len(r) < MAX_COLS {
            self.insert(r, c + 1, id);
            true
        } else {
            self.push(id)
        }
    }

    pub fn split_down(&mut self, target: PaneId, id: PaneId) -> bool {
        let Some((r, c)) = self.pos(target).filter(|_| !self.is_full()) else { return false };
        if r + 1 < MAX_ROWS && self.row_len(r + 1) < MAX_COLS {
            let n = self.row_len(r + 1);
            self.insert(r + 1, c.min(n), id);
            true
        } else {
            self.push(id)
        }
    }

    pub fn remove(&mut self, id: PaneId) {
        if let Some((r, c)) = self.pos(id) {
            self.rows[r].remove(c);
            self.reset();
        }
    }
}
