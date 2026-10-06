use serde::{Deserialize, Serialize};

pub type PaneId = u64;

pub const MAX_COLS: usize = 4;
pub const MAX_ROWS: usize = 2;
pub const MAX_PANES: usize = MAX_COLS * MAX_ROWS;

/// Wo eine gezogene Pane relativ zur Ziel-Pane landet.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Side {
    Left,
    Right,
    Top,
    Bottom,
    /// Plätze tauschen
    Center,
}

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

    /// Verschiebt eine Pane dieses Rasters neben `target`. Die übrigen Panes rücken nach.
    pub fn move_pane(&mut self, id: PaneId, target: PaneId, side: Side) -> bool {
        if id == target {
            return false;
        }
        let (Some((r1, c1)), Some((r2, c2))) = (self.pos(id), self.pos(target)) else { return false };
        if side == Side::Center {
            self.rows[r1][c1] = target;
            self.rows[r2][c2] = id;
            return true;
        }
        let backup = self.clone();
        self.rows[r1].remove(c1);
        self.rows.retain(|r| !r.is_empty());
        if self.place(id, target, side) {
            true
        } else {
            *self = backup;
            false
        }
    }

    /// Fügt eine fremde Pane (z.B. aus einer anderen Session) neben `target` ein.
    pub fn place(&mut self, id: PaneId, target: PaneId, side: Side) -> bool {
        let Some((r, c)) = self.pos(target).filter(|_| !self.is_full() && !self.contains(id)) else { return false };
        let rows = self.rows.len();
        match side {
            Side::Left | Side::Right if self.row_len(r) < MAX_COLS => {
                self.insert(r, if side == Side::Left { c } else { c + 1 }, id);
            }
            Side::Top if r == 1 && self.row_len(0) < MAX_COLS => self.insert(0, c.min(self.row_len(0)), id),
            Side::Top if r == 0 && rows < MAX_ROWS => {
                self.rows.insert(0, vec![id]);
                self.reset();
            }
            Side::Bottom if r == 0 && rows < MAX_ROWS => {
                self.rows.push(vec![id]);
                self.reset();
            }
            Side::Bottom if r == 0 && self.row_len(1) < MAX_COLS => self.insert(1, c.min(self.row_len(1)), id),
            Side::Center => return self.push(id),
            _ => return false,
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(rows: &[&[PaneId]]) -> Grid {
        let mut g = Grid { rows: rows.iter().map(|r| r.to_vec()).collect(), ..Default::default() };
        g.reset();
        g
    }

    #[test]
    fn move_within_row() {
        let mut g = grid(&[&[1, 2, 3]]);
        assert!(g.move_pane(1, 3, Side::Right));
        assert_eq!(g.rows, vec![vec![2, 3, 1]]);
        assert!(g.move_pane(1, 2, Side::Center));
        assert_eq!(g.rows, vec![vec![1, 3, 2]]);
    }

    #[test]
    fn move_to_new_row() {
        let mut g = grid(&[&[1, 2, 3]]);
        assert!(g.move_pane(3, 1, Side::Bottom));
        assert_eq!(g.rows, vec![vec![1, 2], vec![3]]);
        assert!(g.move_pane(1, 3, Side::Left));
        assert_eq!(g.rows, vec![vec![2], vec![1, 3]]);
        assert_eq!(g.widths.len(), g.rows.len());
    }

    #[test]
    fn moving_last_pane_of_row_up() {
        let mut g = grid(&[&[1], &[2]]);
        assert!(g.move_pane(2, 1, Side::Top));
        assert_eq!(g.rows, vec![vec![2], vec![1]]);
    }

    #[test]
    fn full_row_is_rejected_and_restored() {
        let mut g = grid(&[&[1, 2, 3, 4], &[5]]);
        assert!(!g.move_pane(5, 1, Side::Left));
        assert_eq!(g.rows, vec![vec![1, 2, 3, 4], vec![5]]);
    }

    #[test]
    fn place_from_other_session() {
        let mut g = grid(&[&[1]]);
        assert!(g.place(9, 1, Side::Left));
        assert_eq!(g.rows, vec![vec![9, 1]]);
        assert!(!g.place(9, 1, Side::Right));
    }
}
