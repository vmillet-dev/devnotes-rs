//! A table edited from its tools, shown while the caret is in one (the reference's
//! `TABLE_TOOLS`): a row or a column added after the caret's, the caret's row or column taken
//! out, the table taken out. A caret in a cell is the table's path, its row and its column.

use crate::model::{Block, Cell, Document, Position, Selection};
use crate::tree;

/// A new table: a header row and two rows, three columns, as the reference inserted one.
pub fn insert(document: &mut Document, selection: &Selection) -> Selection {
    let after = crate::edit::after_top(document, selection);
    let row = || vec![Cell::new(); 3];
    document
        .blocks
        .insert(after, Block::Table(vec![row(), row(), row()]));
    caret(vec![after, 0, 0])
}

fn caret(path: Vec<usize>) -> Selection {
    Selection::caret(Position { path, offset: 0 })
}

/// The table the caret is in: its path, the caret's row and column.
pub fn cell_of(document: &Document, path: &[usize]) -> Option<(Vec<usize>, usize, usize)> {
    let (table, at) = path.split_at(path.len().checked_sub(2)?);
    matches!(tree::block(document, table), Some(Block::Table(_)))
        .then(|| (table.to_vec(), at[0], at[1]))
}

fn rows_mut<'a>(document: &'a mut Document, table: &[usize]) -> Option<&'a mut Vec<Vec<Cell>>> {
    let (blocks, index) = tree::siblings_mut(document, table)?;
    match &mut blocks[index] {
        Block::Table(rows) => Some(rows),
        _ => None,
    }
}

pub fn add_row_after(document: &mut Document, selection: &Selection) -> Selection {
    let Some((table, row, column)) = cell_of(document, &selection.head.path) else {
        return selection.clone();
    };
    if let Some(rows) = rows_mut(document, &table) {
        let width = rows[row].len();
        rows.insert(row + 1, vec![Cell::new(); width]);
    }
    caret([table.as_slice(), &[row + 1, column]].concat())
}

pub fn add_column_after(document: &mut Document, selection: &Selection) -> Selection {
    let Some((table, row, column)) = cell_of(document, &selection.head.path) else {
        return selection.clone();
    };
    if let Some(rows) = rows_mut(document, &table) {
        for cells in rows.iter_mut() {
            cells.insert((column + 1).min(cells.len()), Cell::new());
        }
    }
    caret([table.as_slice(), &[row, column + 1]].concat())
}

pub fn delete_row(document: &mut Document, selection: &Selection) -> Selection {
    let Some((table, row, column)) = cell_of(document, &selection.head.path) else {
        return selection.clone();
    };
    let left = rows_mut(document, &table).map_or(0, |rows| {
        rows.remove(row);
        rows.len()
    });
    if left == 0 {
        return delete(document, selection);
    }
    let row = row.min(left - 1);
    clamped(document, [table.as_slice(), &[row, column]].concat())
}

pub fn delete_column(document: &mut Document, selection: &Selection) -> Selection {
    let Some((table, row, column)) = cell_of(document, &selection.head.path) else {
        return selection.clone();
    };
    let mut left = 0;
    if let Some(rows) = rows_mut(document, &table) {
        for cells in rows.iter_mut() {
            if column < cells.len() {
                cells.remove(column);
            }
            left = left.max(cells.len());
        }
    }
    if left == 0 {
        return delete(document, selection);
    }
    clamped(
        document,
        [table.as_slice(), &[row, column.min(left - 1)]].concat(),
    )
}

/// The whole table out; the caret where it stood, on the next text.
pub fn delete(document: &mut Document, selection: &Selection) -> Selection {
    let Some((table, _, _)) = cell_of(document, &selection.head.path) else {
        return selection.clone();
    };
    if let Some((blocks, index)) = tree::siblings_mut(document, &table) {
        blocks.remove(index);
        if blocks.is_empty() {
            blocks.push(Block::Paragraph(Vec::new()));
        }
    }
    clamped(document, table)
}

fn clamped(document: &Document, path: Vec<usize>) -> Selection {
    Selection::caret(tree::clamp(document, &Position { path, offset: 0 }))
}

/// Tab in a cell: the next one, a new row past the last; Shift+Tab the one before.
pub fn step(document: &mut Document, selection: &Selection, back: bool) -> Selection {
    let Some((table, row, column)) = cell_of(document, &selection.head.path) else {
        return selection.clone();
    };
    let Some(Block::Table(rows)) = tree::block(document, &table) else {
        return selection.clone();
    };
    let width = rows[row].len();
    let (row, column) = match (back, column + 1 < width, column > 0) {
        (false, true, _) => (row, column + 1),
        (false, false, _) if row + 1 < rows.len() => (row + 1, 0),
        (false, false, _) => {
            return add_row_after(document, &caret([table.as_slice(), &[row, 0]].concat()));
        }
        (true, _, true) => (row, column - 1),
        (true, _, false) if row > 0 => (row - 1, rows[row - 1].len().saturating_sub(1)),
        (true, _, false) => return selection.clone(),
    };
    caret([table.as_slice(), &[row, column]].concat())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{read, write};

    fn at(path: &[usize]) -> Selection {
        caret(path.to_vec())
    }

    #[test]
    fn rows_and_columns_come_and_go_and_the_table_with_them() {
        let mut document = read("| a | b |\n| --- | --- |\n| 1 | 2 |");
        let after = add_row_after(&mut document, &at(&[0, 1, 1]));
        assert_eq!(after, at(&[0, 2, 1]));
        let after = add_column_after(&mut document, &at(&[0, 0, 0]));
        assert_eq!(after, at(&[0, 0, 1]));
        assert_eq!(
            write(&document),
            "| a   |     | b   |\n| --- | --- | --- |\n| 1   |     | 2   |\n|     |     |     |"
        );
        delete_column(&mut document, &at(&[0, 0, 1]));
        delete_row(&mut document, &at(&[0, 2, 0]));
        assert_eq!(
            write(&document),
            "| a   | b   |\n| --- | --- |\n| 1   | 2   |"
        );
        delete(&mut document, &at(&[0, 1, 0]));
        assert_eq!(write(&document), "");
    }

    #[test]
    fn tab_walks_the_cells_and_adds_a_row_past_the_last() {
        let mut document = read("| a | b |\n| --- | --- |\n| 1 | 2 |");
        assert_eq!(step(&mut document, &at(&[0, 0, 1]), false), at(&[0, 1, 0]));
        assert_eq!(step(&mut document, &at(&[0, 1, 0]), true), at(&[0, 0, 1]));
        assert_eq!(step(&mut document, &at(&[0, 1, 1]), false), at(&[0, 2, 0]));
        assert_eq!(
            write(&document),
            "| a   | b   |\n| --- | --- |\n| 1   | 2   |\n|     |     |"
        );
    }
}
