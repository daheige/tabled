#![warn(rust_2018_idioms, missing_debug_implementations, unreachable_pub)]
#![deny(unused_must_use)]

//! Papergrid is a library for generating text-based tables for display
//!
//! # Example
//! ```rust
//! use papergrid::{Grid, Entity, Borders, Settings};
//!
//! let mut grid = Grid::new(2, 2);
//! grid.set_borders(Borders {
//!     top: Some('-'),
//!     top_left: Some('+'),
//!     top_right: Some('+'),
//!     top_intersection: Some('+'),
//!     bottom: Some('-'),
//!     bottom_left: Some('+'),
//!     bottom_right: Some('+'),
//!     bottom_intersection: Some('+'),
//!     horizontal: Some('-'),
//!     horizontal_left: Some('+'),
//!     horizontal_right: Some('+'),
//!     vertical_left: Some('|'),
//!     vertical_right: Some('|'),
//!     vertical_intersection: Some('|'),
//!     intersection: Some('+'),
//! });
//!
//! grid.set(Entity::Cell(0, 0), Settings::new().text("0-0"));
//! grid.set(Entity::Cell(0, 1), Settings::new().text("0-1"));
//! grid.set(Entity::Cell(1, 0), Settings::new().text("1-0"));
//! grid.set(Entity::Cell(1, 1), Settings::new().text("1-1"));
//!
//! assert_eq!(
//!     grid.to_string(),
//!     concat!(
//!         "+---+---+\n",
//!         "|0-0|0-1|\n",
//!         "+---+---+\n",
//!         "|1-0|1-1|\n",
//!         "+---+---+",
//!     )
//! );
//! ```

use std::{
    borrow::Cow,
    cmp,
    collections::{HashMap, HashSet},
    fmt::{self, Display, Write},
    hash::Hash,
};

const DEFAULT_BORDER_HORIZONTAL_CHAR: char = ' ';
const DEFAULT_BORDER_HORIZONTAL_SYMBOL: char = ' ';
const DEFAULT_BORDER_VERTICAL_SYMBOL: char = ' ';
const DEFAULT_BORDER_HORIZONTAL_SYMBOL_REF: &char = &DEFAULT_BORDER_VERTICAL_SYMBOL;
const DEFAULT_BORDER_VERTICAL_SYMBOL_REF: &char = &DEFAULT_BORDER_VERTICAL_SYMBOL;
const DEFAULT_BORDER_INTERSECTION_SYMBOL_REF: &char = &DEFAULT_BORDER_VERTICAL_SYMBOL;
const DEFAULT_INDENT_FILL_CHAR: char = ' ';

/// Grid provides a set of methods for building a text-based table
#[derive(Debug, Clone)]
pub struct Grid {
    size: (usize, usize),
    cells: Vec<Vec<String>>,
    margin: Margin,
    borders: BordersConfig<char>,
    override_split_lines: HashMap<usize, String>,
    spans: HashMap<Position, usize>,
    config: GridConfig,
    #[cfg(feature = "color")]
    border_colors: BordersConfig<BorderColor>,
}

#[derive(Debug, Clone)]
struct GridConfig {
    tab_width: usize,
    styles: StyleConfig,
}

#[derive(Debug, Clone)]
struct StyleConfig {
    global: StyleMap,
    padding: HashMap<Entity, Padding>,
    alignment_horizontal: HashMap<Entity, AlignmentHorizontal>,
    alignment_vertical: HashMap<Entity, AlignmentVertical>,
    formatting: HashMap<Entity, Formatting>,
}

#[derive(Debug, Clone)]
struct StyleMap {
    padding: Padding,
    alignment_horizontal: AlignmentHorizontal,
    alignment_vertical: AlignmentVertical,
    formatting: Formatting,
}

impl Grid {
    /// The new method creates a grid instance with default styles.
    ///
    /// The size of the grid can not be changed after the instance is created.
    ///
    /// # Example
    ///
    /// ```
    /// use papergrid::{Grid, Entity, Settings, Border};
    ///
    ///
    /// let mut grid = Grid::new(2, 2);
    ///
    /// grid.set(Entity::Global, Settings::new().text("Hello World").border(Border::default().right(' ')));
    ///
    /// assert_eq!(
    ///     grid.to_string(),
    ///     "Hello World Hello World \n\
    ///      Hello World Hello World "
    /// );
    /// ```
    ///
    /// Not empty initialization but empty content
    ///
    /// ```rust
    /// use papergrid::Grid;
    ///
    /// let mut grid = Grid::new(2, 2);
    /// assert_eq!(grid.to_string(), "\n");
    /// ```
    ///
    /// Empty
    ///
    /// ```rust
    /// use papergrid::Grid;
    ///
    /// let mut grid = Grid::new(0, 0);
    /// assert_eq!(grid.to_string(), "");
    /// ```
    pub fn new(rows: usize, columns: usize) -> Self {
        let config = GridConfig {
            tab_width: 4,
            styles: StyleConfig {
                global: StyleMap {
                    alignment_horizontal: AlignmentHorizontal::Left,
                    alignment_vertical: AlignmentVertical::Top,
                    padding: Padding::default(),
                    formatting: Formatting::default(),
                },
                alignment_horizontal: HashMap::new(),
                alignment_vertical: HashMap::new(),
                formatting: HashMap::new(),
                padding: HashMap::new(),
            },
        };

        Grid {
            size: (rows, columns),
            cells: vec![vec![String::new(); columns]; rows],
            margin: Margin::default(),
            spans: HashMap::new(),
            config,
            override_split_lines: HashMap::new(),
            borders: BordersConfig::default(),
            #[cfg(feature = "color")]
            border_colors: BordersConfig::default(),
        }
    }

    /// Set method is responsible for modification of cell/row/column.
    ///
    /// The method panics if incorrect cell/row/column index is given.
    ///
    /// # Example
    ///
    /// ```rust
    /// use papergrid::{Grid, Entity, Settings, Borders};
    ///
    /// let mut grid = Grid::new(2, 2);
    ///
    /// grid.set_borders(Borders {
    ///     vertical_intersection: Some('|'),
    ///     horizontal: Some('-'),
    ///     ..Default::default()
    /// });
    ///
    /// grid.set(Entity::Row(0), Settings::new().text("row 1"));
    /// grid.set(Entity::Row(1), Settings::new().text("row 2"));
    /// assert_eq!(
    ///      grid.to_string(),
    ///      "row 1|row 1\n\
    ///       ----- -----\n\
    ///       row 2|row 2"
    /// )
    /// ```
    pub fn set(&mut self, entity: Entity, settings: Settings) {
        if let Some(padding) = settings.padding {
            self.set_padding(entity, padding);
        }

        if let Some(alignment) = settings.alignment_horizontal {
            self.set_alignment_horizontal(entity, alignment);
        }

        if let Some(alignment) = settings.alignment_vertical {
            self.set_alignment_vertical(entity, alignment);
        }

        if let Some(formatting) = settings.formatting {
            self.set_formatting(entity, formatting);
        }

        if let Some(text) = settings.text {
            self.set_text(entity, &text);
        }

        if let Some(border) = settings.border {
            self.set_border(entity, border);
        }

        if let Some(span) = settings.span {
            self.set_span(entity, span);
        }
    }

    /// Set a [`Margin`] value.
    pub fn margin(&mut self, margin: Margin) {
        self.margin = margin;
    }

    /// Returns a [`Margin`] value currently set.
    pub fn get_margin(&self) -> &Margin {
        &self.margin
    }

    /// Clears all theme changes.
    /// And sets it to default.
    pub fn clear_theme(&mut self) {
        self.borders = BordersConfig::default();
        self.override_split_lines.clear();
    }

    /// Set the [`Borders`] value as currect one.
    pub fn set_borders(&mut self, borders: Borders<char>) {
        self.borders.set_borders(borders);
    }

    /// Set the [`Borders`] value as currect one.
    pub fn set_tab_width(&mut self, width: usize) {
        self.config.tab_width = width;
    }

    /// Returns a current [`Borders`] structure.
    pub fn get_borders(&self) -> &Borders<char> {
        &self.borders.borders
    }

    /// Set border set a border value to all cells in [`Entity`].
    pub fn set_border(&mut self, entity: Entity, border: Border) {
        entity
            .iter(self.count_rows(), self.count_columns())
            .for_each(|pos| self.borders.insert_border(pos, border.clone()));
    }

    /// Sets off all borders possible on the [`Entity`].
    ///
    /// It doesn't changes globaly set borders through [`Grid::set_borders`].
    pub fn remove_border(&mut self, entity: Entity) {
        entity
            .iter(self.count_rows(), self.count_columns())
            .for_each(|pos| self.borders.remove_border(pos, self.count_columns()));
    }

    /// Set the border line by row index.
    ///
    /// Row `0` means the top row.
    /// Row `grid.count_rows()` means the bottom row.
    pub fn set_split_line(&mut self, row: usize, line: Line<char>) {
        self.borders.insert_line(row, line);
    }

    /// This function returns a settings of a cell
    pub fn get_settings(&self, row: usize, col: usize) -> Settings {
        let style = self.style(Entity::Cell(row, col));
        let content = &self.cells[row][col];
        let border = self.get_border((row, col));
        let span = self.get_column_span((row, col));

        Settings {
            text: Some(content.clone()),
            padding: Some(Padding {
                left: style.padding.left,
                right: style.padding.right,
                top: style.padding.top,
                bottom: style.padding.bottom,
            }),
            border: Some(border),
            alignment_horizontal: Some(style.alignment_horizontal),
            alignment_vertical: Some(style.alignment_vertical),
            limits: style.limits,
            formatting: None,
            span,
        }
    }

    /// Returns a border of a cell.
    pub fn get_border(&self, (row, col): Position) -> Border<char> {
        let mut border = self
            .borders
            .get_border((row, col), self.count_rows(), self.count_columns())
            .cloned();

        // make sure that there's no user defined lines
        // in which case we use spaces.

        let mut top_set = border.top.is_some();
        let mut bottom_set = border.bottom.is_some();
        let mut left_set = border.left.is_some();
        let mut right_set = border.right.is_some();

        if border.top.is_none() && has_horizontal(self, row) {
            border.top = Some(DEFAULT_BORDER_HORIZONTAL_SYMBOL);
            top_set = true;
        }

        if border.bottom.is_none() && has_horizontal(self, row + 1) {
            border.bottom = Some(DEFAULT_BORDER_HORIZONTAL_SYMBOL);
            bottom_set = true;
        }

        if border.left.is_none() && has_vertical(self, col) {
            border.left = Some(DEFAULT_BORDER_VERTICAL_SYMBOL);
            left_set = true;
        }

        if border.right.is_none() && has_vertical(self, col + 1) {
            border.right = Some(DEFAULT_BORDER_VERTICAL_SYMBOL);
            right_set = true;
        }

        if border.left_top_corner.is_none() && top_set && left_set {
            border.left_top_corner = Some(DEFAULT_BORDER_VERTICAL_SYMBOL);
        }

        if border.left_bottom_corner.is_none() && bottom_set && left_set {
            border.left_bottom_corner = Some(DEFAULT_BORDER_VERTICAL_SYMBOL);
        }

        if border.right_top_corner.is_none() && top_set && right_set {
            border.right_top_corner = Some(DEFAULT_BORDER_VERTICAL_SYMBOL);
        }

        if border.right_bottom_corner.is_none() && bottom_set && right_set {
            border.right_bottom_corner = Some(DEFAULT_BORDER_VERTICAL_SYMBOL);
        }

        border
    }

    pub fn style(&self, entity: Entity) -> Style {
        let styles = &self.config.styles;
        let padding = lookup_entity_value(&styles.padding, styles.global.padding, entity);
        let formatting = lookup_entity_value(&styles.formatting, styles.global.formatting, entity);
        let alignment_horizontal = lookup_entity_value(
            &styles.alignment_horizontal,
            styles.global.alignment_horizontal,
            entity,
        );
        let alignment_vertical = lookup_entity_value(
            &styles.alignment_vertical,
            styles.global.alignment_vertical,
            entity,
        );

        Style {
            padding,
            alignment_vertical,
            alignment_horizontal,
            formatting,
            limits: None,
        }
    }

    /// This function returns content without any style changes
    pub fn get_cell_content(&self, row: usize, column: usize) -> &str {
        self.cells[row][column].as_str()
    }

    /// This function returns content with style changes
    pub fn get_cell_content_formatted(&self, row: usize, column: usize) -> String {
        let text = self.get_cell_content(row, column);
        replace_tab(text, self.config.tab_width)
    }

    /// This function returns a string width.
    pub fn get_string_width(&self, row: usize, column: usize) -> usize {
        string_width_multiline_tab(self.cells[row][column].as_str(), self.config.tab_width)
    }

    /// This function returns an amount of rows on the grid
    pub fn count_rows(&self) -> usize {
        self.size.0
    }

    /// This function returns an amount of columns on the grid
    pub fn count_columns(&self) -> usize {
        self.size.1
    }

    /// Set text value to all cells in [`Entity`].
    pub fn set_text(&mut self, entity: Entity, text: &str) {
        self._set_text(entity, text);
    }

    /// Creates a new Grid from original,
    /// Coping the things like styles and borders.
    ///
    /// It doesn't copy specifics of cells.
    pub fn resize(&self, count_rows: usize, count_cols: usize) -> Self {
        let mut new = Self::new(count_rows, count_cols);
        new.borders.borders = self.borders.borders.clone();
        new.borders.global = self.borders.global;
        new.margin = self.margin;
        new.config.tab_width = self.config.tab_width;
        new.config.styles.global = self.config.styles.global.clone();

        #[cfg(feature = "color")]
        {
            new.border_colors.borders = self.border_colors.borders.clone();
            new.border_colors.global = self.border_colors.global.clone();
        }

        new
    }

    /// Returns a total width of table, including split lines.
    pub fn total_width(&self) -> usize {
        if self.count_rows() == 0 || self.count_columns() == 0 {
            return 0;
        }

        let cells = cells_content(self);
        let widths = columns_width(self, &cells);

        total_width(self, &widths, &self.margin)
    }

    /// Override the split line with a custom text.
    ///
    /// If borders are not set the string won't be rendered.
    pub fn override_split_line(&mut self, row: usize, line: impl Into<String>) {
        self.override_split_lines.insert(row, line.into());
    }

    // hide it by feature?
    // 'private'
    pub fn build_widths(&self) -> Vec<usize> {
        build_widths(self)
    }

    // hide it by feature?
    // 'private'
    pub fn estimate_total_width(&self, widths: &[usize]) -> usize {
        total_width(self, widths, &self.margin)
    }

    // hide it by feature?
    // 'private'
    pub fn build_min_widths(&self) -> Vec<usize> {
        build_min_widths(self)
    }

    /// The function returns all cells by lines.
    ///
    /// It's considered that [`string_width`] on these cells will be the same as the one which will be used in rendering.
    pub fn collect_cells(&self) -> Vec<Vec<Vec<String>>> {
        let count_rows = self.count_rows();
        let count_columns = self.count_columns();

        let mut rows = vec![Vec::with_capacity(self.count_columns()); self.count_rows()];
        (0..count_rows).for_each(|row| {
            (0..count_columns).for_each(|col| {
                let content = replace_tab(&self.cells[row][col], self.config.tab_width);

                // fixme: I guess it can be done in a different place?
                let lines: Vec<_> = content.lines().map(ToOwned::to_owned).collect();
                rows[row].push(lines);
            });
        });

        rows
    }

    /// The function returns whether the cells will be rendered or it will be hidden by a cell with a span.
    pub fn is_cell_visible(&self, pos: Position) -> bool {
        let is_cell_overriden = is_cell_overriden(self, pos);
        !is_cell_overriden
    }

    /// Set a column span to a given cells.
    pub fn set_span(&mut self, entity: Entity, span: usize) {
        entity
            .iter(self.count_rows(), self.count_columns())
            .for_each(|pos| self.set_cell_span(pos, span));
    }

    /// Set a padding to a given cells.
    pub fn set_padding(&mut self, entity: Entity, padding: Padding) {
        set_entity_value(
            &mut self.config.styles.padding,
            &mut self.config.styles.global.padding,
            entity,
            padding,
        );
    }

    /// Set a formatting to a given cells.
    pub fn set_formatting(&mut self, entity: Entity, formatting: Formatting) {
        set_entity_value(
            &mut self.config.styles.formatting,
            &mut self.config.styles.global.formatting,
            entity,
            formatting,
        );
    }

    /// Set a vertical alignment to a given cells.
    pub fn set_alignment_vertical(&mut self, entity: Entity, alignment: AlignmentVertical) {
        set_entity_value(
            &mut self.config.styles.alignment_vertical,
            &mut self.config.styles.global.alignment_vertical,
            entity,
            alignment,
        );
    }

    /// Set a horizontal alignment to a given cells.
    pub fn set_alignment_horizontal(&mut self, entity: Entity, alignment: AlignmentHorizontal) {
        set_entity_value(
            &mut self.config.styles.alignment_horizontal,
            &mut self.config.styles.global.alignment_horizontal,
            entity,
            alignment,
        );
    }

    fn set_cell_span(&mut self, (row, mut col): Position, mut span: usize) {
        if row >= self.count_rows() {
            return;
        }

        // It's a default span so we can do nothing.
        if span == 1 {
            return;
        }

        if col == 0 && span == 0 {
            return;
        }

        if col + span > self.count_columns() {
            span = self.count_columns() - col;
        }

        if span == 0 && col > 0 {
            match closest_visible(self, row, col - 1) {
                Some(c) => {
                    span += 1 + col - c;
                    col = c;
                }
                None => return,
            }
        }

        self.spans.insert((row, col), span);
    }

    fn _set_text(&mut self, entity: Entity, text: &str) {
        entity
            .iter(self.count_rows(), self.count_columns())
            .for_each(|(row, col)| {
                self.cells[row][col] = text.to_owned();
            });
    }

    /// Get a span value of the cell, if any is set.
    pub fn get_column_span(&self, (row, col): Position) -> Option<usize> {
        self.spans.get(&(row, col)).copied()
    }

    /// Get a span value of the cell, if any is set.
    pub fn iter_column_spans(&self) -> impl Iterator<Item = (Position, usize)> + '_ {
        self.spans.iter().map(|(&pos, &span)| (pos, span))
    }

    /// Verifies if there's any spans set.
    pub fn has_column_spans(&self) -> bool {
        !self.spans.is_empty()
    }
}

#[cfg(feature = "color")]
impl Grid {
    pub fn get_color_borders(&self) -> &Borders<BorderColor> {
        &self.border_colors.borders
    }

    pub fn get_colored_border(&self, pos: Position) -> Border<Symbol> {
        let b = self.get_border(pos);
        let cls = self
            .border_colors
            .get_border(pos, self.count_rows(), self.count_columns());

        Border {
            top: b.top.map(|c| Symbol::new(c, cls.top.cloned())),
            bottom: b.bottom.map(|c| Symbol::new(c, cls.bottom.cloned())),
            left: b.left.map(|c| Symbol::new(c, cls.left.cloned())),
            right: b.right.map(|c| Symbol::new(c, cls.right.cloned())),
            left_top_corner: b
                .left_top_corner
                .map(|c| Symbol::new(c, cls.left_top_corner.cloned())),
            left_bottom_corner: b
                .left_bottom_corner
                .map(|c| Symbol::new(c, cls.left_bottom_corner.cloned())),
            right_top_corner: b
                .right_top_corner
                .map(|c| Symbol::new(c, cls.right_top_corner.cloned())),
            right_bottom_corner: b
                .right_bottom_corner
                .map(|c| Symbol::new(c, cls.right_bottom_corner.cloned())),
        }
    }

    pub fn set_border_color(&mut self, clr: BorderColor) {
        self.border_colors = BordersConfig::default();
        self.border_colors.global = Some(clr);
    }

    pub fn set_borders_color(&mut self, clrs: Borders<BorderColor>) {
        self.border_colors.borders = clrs;
    }

    pub fn set_colored_border(&mut self, entity: Entity, border: Border<Symbol>) {
        let border_chars = symbol_border_into_border(&border);
        self.set_border(entity, border_chars);

        let border = symbol_border_into_color_border(border);
        self.set_border_color_(entity, border);
    }

    fn set_border_color_(&mut self, entity: Entity, border: Border<BorderColor>) {
        entity
            .iter(self.count_rows(), self.count_columns())
            .for_each(|pos| self.border_colors.insert_border(pos, border.clone()))
    }
}

#[cfg(feature = "color")]
fn symbol_border_into_border(border: &Border<Symbol>) -> Border {
    Border {
        top: border.top.as_ref().map(|s| s.c),
        bottom: border.bottom.as_ref().map(|s| s.c),
        left: border.left.as_ref().map(|s| s.c),
        left_top_corner: border.left_top_corner.as_ref().map(|s| s.c),
        left_bottom_corner: border.left_bottom_corner.as_ref().map(|s| s.c),
        right: border.right.as_ref().map(|s| s.c),
        right_top_corner: border.right_top_corner.as_ref().map(|s| s.c),
        right_bottom_corner: border.right_bottom_corner.as_ref().map(|s| s.c),
    }
}

#[cfg(feature = "color")]
fn symbol_border_into_color_border(border: Border<Symbol>) -> Border<BorderColor> {
    Border {
        top: border.top.and_then(|s| s.ansi_sequences),
        bottom: border.bottom.and_then(|s| s.ansi_sequences),
        left: border.left.and_then(|s| s.ansi_sequences),
        left_top_corner: border.left_top_corner.and_then(|s| s.ansi_sequences),
        left_bottom_corner: border.left_bottom_corner.and_then(|s| s.ansi_sequences),
        right: border.right.and_then(|s| s.ansi_sequences),
        right_top_corner: border.right_top_corner.and_then(|s| s.ansi_sequences),
        right_bottom_corner: border.right_bottom_corner.and_then(|s| s.ansi_sequences),
    }
}

impl fmt::Display for Grid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.count_rows() == 0 || self.count_columns() == 0 {
            return Ok(());
        }

        let cells = cells_content(self);

        let heights = rows_height(self, &cells);
        let widths = columns_width(self, &cells);

        print_grid(f, self, &widths, heights, &cells)
    }
}

/// Entity a structure which represent a set of cells.
#[derive(PartialEq, Eq, Debug, Hash, Clone, Copy)]
pub enum Entity {
    /// All cells on the grid.
    Global,
    /// All cells in a column on the grid.
    Column(usize),
    /// All cells in a row on the grid.
    Row(usize),
    /// A particular cell (row, column) on the grid.
    Cell(usize, usize),
}

impl Entity {
    /// Iterate over cells which are covered via the [`Entity`].
    pub fn iter(&self, count_rows: usize, count_cols: usize) -> EntityIterator {
        EntityIterator {
            entity: *self,
            count_rows,
            count_cols,
            i: 0,
            j: 0,
        }
    }
}

/// An iterator over cells.
///
/// Produced from [`Entity::iter`].
#[derive(Debug)]
pub struct EntityIterator {
    entity: Entity,
    count_rows: usize,
    count_cols: usize,
    i: usize,
    j: usize,
}

impl Iterator for EntityIterator {
    type Item = Position;

    fn next(&mut self) -> Option<Self::Item> {
        if self.count_rows == 0 || self.count_cols == 0 {
            return None;
        }

        match self.entity {
            Entity::Cell(row, col) => {
                self.count_cols = 0;
                self.count_rows = 0;

                Some((row, col))
            }
            Entity::Column(col) => {
                if self.i >= self.count_rows {
                    return None;
                }

                let i = self.i;
                self.i += 1;

                Some((i, col))
            }
            Entity::Row(row) => {
                if self.j >= self.count_cols {
                    return None;
                }

                let j = self.j;
                self.j += 1;

                Some((row, j))
            }
            Entity::Global => {
                if self.j >= self.count_cols {
                    self.j = 0;
                    self.i += 1;

                    if self.i >= self.count_rows {
                        return None;
                    }
                }

                let j = self.j;
                self.j += 1;

                Some((self.i, j))
            }
        }
    }
}

/// Settings represent setting of a particular cell
#[derive(Debug, Clone, Default)]
pub struct Settings {
    text: Option<String>,
    padding: Option<Padding>,
    border: Option<Border>,
    span: Option<usize>,
    alignment_horizontal: Option<AlignmentHorizontal>,
    alignment_vertical: Option<AlignmentVertical>,
    formatting: Option<Formatting>,
    limits: Option<Limits>,
}

impl Settings {
    /// New method constructs an instance of settings
    pub fn new() -> Self {
        Self::default()
    }

    /// Text method sets content for a cell
    pub fn text<S: Into<String>>(mut self, text: S) -> Self {
        self.text = Some(text.into());
        self
    }

    /// padding method sets padding for a cell
    pub fn padding(mut self, left: Indent, right: Indent, top: Indent, bottom: Indent) -> Self {
        self.padding = Some(Padding {
            top,
            bottom,
            left,
            right,
        });
        self
    }

    /// Alignment method sets horizontal alignment for a cell
    pub fn alignment(mut self, alignment: AlignmentHorizontal) -> Self {
        self.alignment_horizontal = Some(alignment);
        self
    }

    /// Alignment method sets horizontal alignment for a cell
    pub fn vertical_alignment(mut self, alignment: AlignmentVertical) -> Self {
        self.alignment_vertical = Some(alignment);
        self
    }

    /// Set the settings's span.
    pub fn span(mut self, span: usize) -> Self {
        self.span = Some(span);
        self
    }

    /// Set the settings's border.
    ///
    /// The border setting is in a restrictive manner, by default.
    /// So if there was no split line but border relies on it
    /// a error will be issued.
    pub fn border(mut self, border: Border) -> Self {
        self.border = Some(border);
        self
    }

    /// Set a formatting settings.
    ///
    /// It overades them even if any were not set.
    pub fn formatting(mut self, formatting: Formatting) -> Self {
        self.formatting = Some(formatting);
        self
    }

    /// Set a formatting settings.
    ///
    /// It overades them even if any were not set.
    pub fn width(mut self, value: usize, fill: char) -> Self {
        self.limits = Some(Limits { fill, width: value });
        self
    }
}

/// Border is a representation of a cells's borders (left, right, top, bottom, and the corners)
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct Border<T = char> {
    pub top: Option<T>,
    pub bottom: Option<T>,
    pub left: Option<T>,
    pub right: Option<T>,
    pub left_top_corner: Option<T>,
    pub left_bottom_corner: Option<T>,
    pub right_top_corner: Option<T>,
    pub right_bottom_corner: Option<T>,
}

impl<T> Border<T> {
    /// This function constructs a cell borders with all sides set.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        top: T,
        bottom: T,
        left: T,
        right: T,
        top_left: T,
        top_right: T,
        bottom_left: T,
        bottom_right: T,
    ) -> Self {
        Self {
            top: Some(top),
            bottom: Some(bottom),
            right: Some(right),
            right_top_corner: Some(top_right),
            right_bottom_corner: Some(bottom_right),
            left: Some(left),
            left_bottom_corner: Some(bottom_left),
            left_top_corner: Some(top_left),
        }
    }

    /// Set a top border character.
    pub fn top(mut self, c: T) -> Self {
        self.top = Some(c);
        self
    }

    /// Set a bottom border character.
    pub fn bottom(mut self, c: T) -> Self {
        self.bottom = Some(c);
        self
    }

    /// Set a left border character.
    pub fn left(mut self, c: T) -> Self {
        self.left = Some(c);
        self
    }

    /// Set a right border character.
    pub fn right(mut self, c: T) -> Self {
        self.right = Some(c);
        self
    }

    /// Set a top left intersection character.
    pub fn top_left_corner(mut self, c: T) -> Self {
        self.left_top_corner = Some(c);
        self
    }

    /// Set a top right intersection character.
    pub fn top_right_corner(mut self, c: T) -> Self {
        self.right_top_corner = Some(c);
        self
    }

    /// Set a bottom left intersection character.
    pub fn bottom_left_corner(mut self, c: T) -> Self {
        self.left_bottom_corner = Some(c);
        self
    }

    /// Set a bottom right intersection character.
    pub fn bottom_right_corner(mut self, c: T) -> Self {
        self.right_bottom_corner = Some(c);
        self
    }
}

impl<T: Copy> Border<T> {
    /// This function constructs a cell borders with all sides's char set to a given character.
    /// It behaives like [`Border::new`] with the same character set to each side.
    pub fn filled(c: T) -> Self {
        Self::new(c, c, c, c, c, c, c, c)
    }
}

impl<T: Copy> Border<&T> {
    /// This function constructs a cell borders with all sides's char set to a given character.
    /// It behaives like [`Border::new`] with the same character set to each side.
    pub fn cloned(&self) -> Border<T> {
        Border {
            top: self.top.copied(),
            bottom: self.bottom.copied(),
            left: self.left.copied(),
            right: self.right.copied(),
            left_bottom_corner: self.left_bottom_corner.copied(),
            left_top_corner: self.left_top_corner.copied(),
            right_bottom_corner: self.right_bottom_corner.copied(),
            right_top_corner: self.right_top_corner.copied(),
        }
    }
}

/// Style represent a style of a cell on a grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Style {
    pub padding: Padding,
    pub alignment_horizontal: AlignmentHorizontal,
    pub alignment_vertical: AlignmentVertical,
    pub formatting: Formatting,
    pub limits: Option<Limits>,
}

/// Formatting represent a logic of formatting of a cell.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Formatting {
    pub horizontal_trim: bool,
    pub vertical_trim: bool,
    pub allow_lines_alignement: bool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    pub width: usize,
    pub fill: char,
}

/// Margin represent a 4 indents of table as a whole.
#[derive(Default, Debug, Clone, Copy)]
pub struct Margin {
    pub top: Indent,
    pub bottom: Indent,
    pub left: Indent,
    pub right: Indent,
}

/// Padding represent a 4 indents of cell.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Padding {
    pub top: Indent,
    pub bottom: Indent,
    pub left: Indent,
    pub right: Indent,
}

/// Indent represent a filled space.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Indent {
    pub fill: char,
    pub size: usize,
}

impl Indent {
    /// Creates a new Indent structure.
    pub fn new(size: usize, fill: char) -> Self {
        Self { fill, size }
    }

    /// Creates a new Indent startucture with space (`' '`) as a fill character.
    pub fn spaced(size: usize) -> Self {
        Self {
            size,
            fill: DEFAULT_INDENT_FILL_CHAR,
        }
    }
}

impl Default for Indent {
    fn default() -> Self {
        Self {
            fill: DEFAULT_INDENT_FILL_CHAR,
            size: 0,
        }
    }
}

/// [`AlignmentHorizontal`] represents an horizontal alignment of a cell content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignmentHorizontal {
    Center,
    Left,
    Right,
}

fn print_text_formated(
    f: &mut fmt::Formatter<'_>,
    text: &str,
    text_width: usize,
    alignment: AlignmentHorizontal,
    available: usize,
    tab_width: usize,
) -> fmt::Result {
    let diff = available - text_width;
    let (left, right) = match alignment {
        AlignmentHorizontal::Left => (0, diff),
        AlignmentHorizontal::Right => (diff, 0),
        AlignmentHorizontal::Center => {
            let left = diff / 2;
            let rest = diff - left;
            (left, rest)
        }
    };

    repeat_char(f, ' ', left)?;
    print_text(f, text, tab_width)?;
    repeat_char(f, ' ', right)?;

    Ok(())
}

fn print_text(f: &mut fmt::Formatter<'_>, text: &str, tab_width: usize) -> fmt::Result {
    // So to not use replace_tab we are printing by char;
    // Hopefully it's more affective as it reduceses a number of allocations.
    for c in text.chars() {
        match c {
            '\r' => (),
            '\t' => repeat_char(f, ' ', tab_width)?,
            c => f.write_char(c)?,
        }
    }

    Ok(())
}

/// [`AlignmentVertical`] represents an vertical alignment of a cell content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignmentVertical {
    Center,
    Top,
    Bottom,
}

fn indent_from_top(alignment: AlignmentVertical, height: usize, real_height: usize) -> usize {
    match alignment {
        AlignmentVertical::Top => 0,
        AlignmentVertical::Bottom => height - real_height,
        AlignmentVertical::Center => (height - real_height) / 2,
    }
}

fn build_cell_line(
    f: &mut fmt::Formatter<'_>,
    cell: &CellContent<'_>,
    line: usize,
    width: usize,
    height: usize,
    style: &Style,
    tab_width: usize,
) -> fmt::Result {
    let original_cell_height = cell.lines.len();
    let mut cell_height = original_cell_height;

    if style.formatting.vertical_trim {
        cell_height -= count_empty_lines_on_ends(&cell.lines);
    }

    let skip_lines = top_indent(style, cell_height, height);
    if skip_lines > line {
        return repeat_char(f, style.padding.top.fill, width);
    }

    let mut index = line - skip_lines;
    let cell_has_this_line = cell_height > index;
    // happens when other cells have bigger height
    if !cell_has_this_line {
        return repeat_char(f, style.padding.bottom.fill, width);
    }

    if style.formatting.vertical_trim {
        let empty_lines = count_empty_lines_at_start(&cell.lines);
        index += empty_lines;

        if index > original_cell_height {
            return repeat_char(f, style.padding.top.fill, width);
        }
    }

    let width = width - style.padding.left.size - style.padding.right.size;
    repeat_char(f, style.padding.left.fill, style.padding.left.size)?;

    let text = cell.lines.get(index).unwrap_or(&Cow::Borrowed(""));

    build_format_line(
        f,
        index,
        text,
        cell,
        width,
        style.alignment_horizontal,
        style.formatting.horizontal_trim,
        style.formatting.allow_lines_alignement,
        tab_width,
    )?;

    repeat_char(f, style.padding.right.fill, style.padding.right.size)?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn build_format_line(
    f: &mut fmt::Formatter<'_>,
    index: usize,
    line: &str,
    cell: &CellContent<'_>,
    width: usize,
    alignment: AlignmentHorizontal,
    line_trim: bool,
    line_alignement: bool,
    tab_width: usize,
) -> Result<(), fmt::Error> {
    let text = if line_trim {
        string_trim(line)
    } else {
        Cow::Borrowed(line)
    };

    let line_width = if line_trim {
        string_width_tab(&text, tab_width)
    } else {
        cell.lines_width.get(index).copied().unwrap_or(0)
    };

    if line_alignement {
        return print_text_formated(f, &text, line_width, alignment, width, tab_width);
    }

    let cell_width = if line_trim {
        cell.lines
            .iter()
            .map(|line| string_width_tab(line.trim(), tab_width))
            .max()
            .unwrap_or(0)
    } else {
        cell.width
    };

    print_text_formated(f, &text, cell_width, alignment, width, tab_width)?;

    let rest_width = cell_width - line_width;
    repeat_char(f, ' ', rest_width)?;

    Ok(())
}

fn count_empty_lines_on_ends(lines: &[Cow<'_, str>]) -> usize {
    let end_lines = lines
        .iter()
        .rev()
        .take_while(|s| s.trim().is_empty())
        .count();
    let start_lines = lines.iter().take_while(|s| s.trim().is_empty()).count();
    start_lines + end_lines
}

fn count_empty_lines_at_start(lines: &[Cow<'_, str>]) -> usize {
    lines.iter().take_while(|s| s.trim().is_empty()).count()
}

fn top_indent(style: &Style, cell_height: usize, height: usize) -> usize {
    let height = height - style.padding.top.size;
    let indent = indent_from_top(style.alignment_vertical, height, cell_height);

    indent + style.padding.top.size
}

fn repeat_symbol(f: &mut fmt::Formatter<'_>, c: char, n: usize) -> fmt::Result {
    if n > 0 {
        for _ in 0..n {
            c.fmt(f)?;
        }
    }
    Ok(())
}

fn repeat_char(f: &mut fmt::Formatter<'_>, c: char, n: usize) -> fmt::Result {
    if n > 0 {
        for _ in 0..n {
            f.write_char(c)?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Default)]
struct CellContent<'a> {
    lines: Vec<Cow<'a, str>>,
    lines_width: Vec<usize>,
    width: usize,
}

fn cells_content(grid: &Grid) -> Vec<Vec<CellContent<'_>>> {
    let mut cells = vec![vec![CellContent::default(); grid.count_columns()]; grid.count_rows()];

    for (row, cells) in cells.iter_mut().enumerate() {
        for (col, cell) in cells.iter_mut().enumerate() {
            if is_cell_overriden(grid, (row, col)) {
                continue;
            }

            let text = &grid.cells[row][col];

            let count_lines = count_lines(text);

            let mut lines = vec![Cow::Borrowed(""); count_lines];
            let mut widths = vec![0; count_lines];
            let mut max_width = 0;

            for (i, line) in get_lines(text).enumerate() {
                widths[i] = string_width_tab(&line, grid.config.tab_width);
                lines[i] = line;
                max_width = cmp::max(max_width, widths[i]);
            }

            *cell = CellContent {
                lines,
                lines_width: widths,
                width: max_width,
            };
        }
    }

    cells
}

fn build_widths(grid: &Grid) -> Vec<usize> {
    let mut widths = vec![0; grid.count_columns()];
    for (col, column) in widths.iter_mut().enumerate() {
        let max = (0..grid.count_rows())
            .filter(|&row| is_simple_cell(grid, (row, col)))
            .map(|row| get_cell_width(grid, (row, col)))
            .max()
            .unwrap_or(0);

        *column = max;
    }

    adjust_spans_2(grid, &mut widths);

    widths
}

fn adjust_spans_2(grid: &Grid, widths: &mut [usize]) {
    if grid.spans.is_empty() {
        return;
    }

    // The overall width disctribution will be different depend on the order.
    //
    // We sort spans in order to prioritize the smaller spans first.
    let mut spans = grid.spans.iter().collect::<Vec<_>>();
    spans.sort_unstable_by(|a, b| match a.1.cmp(b.1) {
        cmp::Ordering::Equal => a.0.cmp(b.0),
        o => o,
    });

    // todo: the order is matter here; we need to figure out what is correct.
    for (&(row, col), span) in spans {
        adjust_range_2(grid, row, col, col + span, widths);
    }
}

fn adjust_range_2(grid: &Grid, row: usize, start: usize, end: usize, widths: &mut [usize]) {
    let max_span_width = get_cell_width(grid, (row, start));
    let range_width = range_width(grid, start, end, widths);

    if range_width >= max_span_width {
        return;
    }

    inc_range_width(widths, max_span_width - range_width, start, end);
}

fn build_min_widths(grid: &Grid) -> Vec<usize> {
    let mut widths = vec![0; grid.count_columns()];
    for (col, column) in widths.iter_mut().enumerate() {
        let max = (0..grid.count_rows())
            .map(|row| get_cell_padding(grid, (row, col)))
            .max()
            .unwrap_or(0);

        *column = max;
    }

    adjust_spans_min(grid, &mut widths);

    widths
}

fn adjust_spans_min(grid: &Grid, widths: &mut [usize]) {
    if grid.spans.is_empty() {
        return;
    }

    // The overall width disctribution will be different depend on the order.
    //
    // We sort spans in order to prioritize the smaller spans first.
    let mut spans = grid.spans.iter().collect::<Vec<_>>();
    spans.sort_unstable_by(|a, b| match a.1.cmp(b.1) {
        cmp::Ordering::Equal => a.0.cmp(b.0),
        o => o,
    });

    // todo: the order is matter here; we need to figure out what is correct.
    for (&(row, col), span) in spans {
        adjust_range_min(grid, row, col, col + span, widths);
    }
}

fn adjust_range_min(grid: &Grid, row: usize, start: usize, end: usize, widths: &mut [usize]) {
    let max_span_width = get_cell_padding(grid, (row, start));
    let range_width = range_width(grid, start, end, widths);

    if range_width >= max_span_width {
        return;
    }

    inc_range_width(widths, max_span_width - range_width, start, end);
}

fn columns_width(grid: &Grid, cells: &[Vec<CellContent<'_>>]) -> Vec<usize> {
    let mut widths = vec![0; grid.count_columns()];
    for (col, column) in widths.iter_mut().enumerate() {
        let max = (0..grid.count_rows())
            .filter(|&row| is_simple_cell(grid, (row, col)))
            .map(|row| get_cell_width_cells(grid, cells, (row, col)))
            .max()
            .unwrap_or(0);

        *column = max;
    }

    adjust_spans(grid, cells, &mut widths);

    widths
}

fn adjust_spans(grid: &Grid, cells: &[Vec<CellContent<'_>>], widths: &mut [usize]) {
    if grid.spans.is_empty() {
        return;
    }

    // The overall width disctribution will be different depend on the order.
    //
    // We sort spans in order to prioritize the smaller spans first.
    let mut spans = grid.spans.iter().collect::<Vec<_>>();
    spans.sort_unstable_by(|a, b| match a.1.cmp(b.1) {
        cmp::Ordering::Equal => a.0.cmp(b.0),
        o => o,
    });

    // todo: the order is matter here; we need to figure out what is correct.
    for (&(row, col), span) in spans {
        adjust_range(grid, cells, row, col, col + span, widths);
    }
}

fn adjust_range(
    grid: &Grid,
    cells: &[Vec<CellContent<'_>>],
    row: usize,
    start: usize,
    end: usize,
    widths: &mut [usize],
) {
    let max_span_width = get_cell_width_cells(grid, cells, (row, start));
    let range_width = range_width(grid, start, end, widths);

    if range_width >= max_span_width {
        return;
    }

    inc_range_width(widths, max_span_width - range_width, start, end);
}

fn get_cell_width(grid: &Grid, (row, col): Position) -> usize {
    grid.get_string_width(row, col) + get_cell_padding(grid, (row, col))
}

fn get_cell_padding(grid: &Grid, (row, col): Position) -> usize {
    let style = grid.style(Entity::Cell(row, col));
    style.padding.left.size + style.padding.right.size
}

fn get_cell_width_cells(
    grid: &Grid,
    cells: &[Vec<CellContent<'_>>],
    (row, col): Position,
) -> usize {
    let style = grid.style(Entity::Cell(row, col));
    cells[row][col].width + style.padding.left.size + style.padding.right.size
}

fn range_width(grid: &Grid, start: usize, end: usize, widths: &[usize]) -> usize {
    let count_borders = count_borders_in_range(grid, start, end);
    let range_width = widths[start..end].iter().sum::<usize>();
    count_borders + range_width
}

fn is_cell_visible(grid: &Grid, pos: Position) -> bool {
    let is_cell_overriden = is_cell_overriden(grid, pos);
    !is_cell_overriden
}

fn is_cell_overriden(grid: &Grid, pos: Position) -> bool {
    grid.spans
        .iter()
        .any(|(&(row, col), span)| pos.1 > col && pos.1 < col + span && row == pos.0)
}

fn is_simple_cell(grid: &Grid, pos: Position) -> bool {
    let is_spanned = grid
        .spans
        .iter()
        .any(|(&(row, col), span)| pos.1 >= col && pos.1 < col + span && pos.0 == row);

    !is_spanned
}

pub fn count_borders_in_range(grid: &Grid, start: usize, end: usize) -> usize {
    (start..end)
        .skip(1)
        .filter(|&i| has_vertical(grid, i))
        .count()
}

fn inc_range_width(widths: &mut [usize], size: usize, start: usize, end: usize) {
    if widths.is_empty() {
        return;
    }

    let span = end - start;
    let one = size / span;
    let rest = size - span * one;

    let mut i = start;
    while i < end {
        if i == start {
            widths[i] += one + rest;
        } else {
            widths[i] += one;
        }

        i += 1;
    }
}

fn closest_visible(grid: &Grid, row: usize, mut col: usize) -> Option<usize> {
    loop {
        if is_cell_visible(grid, (row, col)) {
            return Some(col);
        }

        if col == 0 {
            return None;
        }

        col -= 1;
    }
}

fn rows_height<'a>(
    grid: &'a Grid,
    cells: &'a [Vec<CellContent<'_>>],
) -> impl Iterator<Item = usize> + 'a {
    (0..grid.count_rows()).map(move |row| {
        (0..grid.count_columns())
            .map(|col| cell_height(grid, cells, (row, col)))
            .max()
            .unwrap_or(0)
    })
}

fn cell_height(grid: &Grid, cells: &[Vec<CellContent<'_>>], pos: Position) -> usize {
    let count_lines = if cells.is_empty() {
        1
    } else {
        cells[pos.0][pos.1].lines.len()
    };

    let style = grid.style(Entity::Cell(pos.0, pos.1));
    count_lines + style.padding.top.size + style.padding.bottom.size
}

fn replace_tab(text: &str, n: usize) -> String {
    // it's a general case which probably must be faster?
    if n == 4 {
        text.replace('\t', "    ")
    } else {
        let mut text = text.to_owned();
        replace_tab_range(&mut text, n);
        text
    }
}

fn replace_tab_range(cell: &mut String, n: usize) -> &str {
    let mut skip = 0;
    while let &Some(pos) = &cell[skip..].find('\t') {
        let pos = skip + pos;

        let is_escaped = pos > 0 && cell.get(pos - 1..pos) == Some("\\");
        if is_escaped {
            skip = pos + 1;
        } else if n == 0 {
            cell.remove(pos);
            skip = pos;
        } else {
            // I'am not sure which version is faster a loop of 'replace'
            // or allacation of a string for replacement;
            cell.replace_range(pos..=pos, &" ".repeat(n));
            skip = pos + 1;
        }

        if cell.is_empty() || skip >= cell.len() {
            break;
        }
    }
    cell
}

// only valid to call for stabilized widths.
fn total_width(grid: &Grid, widths: &[usize], margin: &Margin) -> usize {
    if grid.count_rows() == 0 || grid.count_columns() == 0 {
        return 0;
    }

    let content_width = (0..grid.count_columns())
        .filter(|&col| is_cell_visible(grid, (0, col)))
        .map(|col| grid_cell_width(grid, widths, (0, col)))
        .sum::<usize>();

    let count_borders = (0..grid.count_columns())
        .filter(|&col| is_cell_visible(grid, (0, col)))
        .filter(|&col| has_vertical(grid, col))
        .count()
        + usize::from(has_vertical(grid, grid.count_columns()));

    content_width + count_borders + margin.left.size + margin.right.size
}

/// strip cuts the string to a specific width.
///
/// Width is expected to be in bytes.
pub fn cut_str(s: &str, width: usize) -> Cow<'_, str> {
    const REPLACEMENT: char = '\u{FFFD}';

    #[cfg(feature = "color")]
    {
        let stripped = ansi_str::AnsiStr::ansi_strip(s);
        let (length, count_unknowns, _) = string_split_at_length(&stripped, width);

        let mut buf = ansi_str::AnsiStr::ansi_cut(s, ..length);
        buf.extend(std::iter::repeat(REPLACEMENT).take(count_unknowns));

        Cow::Owned(buf)
    }
    #[cfg(not(feature = "color"))]
    {
        let (length, count_unknowns, _) = string_split_at_length(s, width);
        let buf = &s[..length];
        if count_unknowns == 0 {
            return Cow::Borrowed(buf);
        }

        let mut buf = buf.to_owned();
        buf.extend(std::iter::repeat(REPLACEMENT).take(count_unknowns));

        Cow::Owned(buf)
    }
}

pub fn string_split_at_length(s: &str, width: usize) -> (usize, usize, usize) {
    let mut length = 0;
    let mut i = 0;
    for c in s.chars() {
        if i == width {
            break;
        };

        let c_width = unicode_width::UnicodeWidthChar::width(c).unwrap_or(0);

        // We cut the chars which takes more then 1 symbol to display,
        // in order to archive the necessary width.
        if i + c_width > width {
            let count = width - i;
            return (length, count, c.len_utf8());
        }

        i += c_width;
        length += c.len_utf8();
    }

    (length, 0, 0)
}

/// Returns a string width.
#[cfg(not(feature = "color"))]
pub fn string_width(text: &str) -> usize {
    unicode_width::UnicodeWidthStr::width(text)
}

/// Returns a string width.
#[cfg(feature = "color")]
pub fn string_width(text: &str) -> usize {
    let b = strip_ansi_escapes::strip(text.as_bytes()).unwrap();
    let s = std::str::from_utf8(&b).unwrap();
    unicode_width::UnicodeWidthStr::width(s)
}

/// Returns a max string width of a line.
#[cfg(not(feature = "color"))]
pub fn string_width_multiline(text: &str) -> usize {
    text.lines()
        .map(unicode_width::UnicodeWidthStr::width)
        .max()
        .unwrap_or(0)
}

/// Returns a max string width of a line.
#[cfg(feature = "color")]
pub fn string_width_multiline(text: &str) -> usize {
    let b = strip_ansi_escapes::strip(text.as_bytes()).unwrap();
    let s = std::str::from_utf8(&b).unwrap();

    s.lines()
        .map(unicode_width::UnicodeWidthStr::width)
        .max()
        .unwrap_or(0)
}

fn string_width_tab(text: &str, tab_width: usize) -> usize {
    let width = string_width(text);
    let count_tabs = count_tabs(text);

    width + count_tabs * tab_width
}

fn string_width_multiline_tab(text: &str, tab_width: usize) -> usize {
    text.lines()
        .map(|line| string_width_tab(line, tab_width))
        .max()
        .unwrap_or(0)
}

#[cfg(not(feature = "color"))]
fn string_trim(text: &str) -> Cow<'_, str> {
    text.trim().into()
}

#[cfg(feature = "color")]
fn string_trim(text: &str) -> Cow<'_, str> {
    ansi_str::AnsiStr::ansi_trim(text).into()
}

#[cfg(not(feature = "color"))]
fn get_lines(text: &str) -> impl Iterator<Item = Cow<'_, str>> {
    // we call split but not `lines()` in order to match colored implementation
    text.split('\n').map(Cow::Borrowed)
}

#[cfg(feature = "color")]
fn get_lines(text: &str) -> impl Iterator<Item = Cow<'_, str>> {
    ansi_str::AnsiStr::ansi_split(text, "\n")
}

#[derive(Debug, Clone, Default)]
struct BordersConfig<T> {
    global: Option<T>,
    borders: Borders<T>,
    cells: BordersMap<T>,
    lines: HashMap<usize, Line<T>>,
    layout: BordersLayout,
}

#[derive(Debug, Clone, Default)]
pub struct Borders<T = char> {
    pub top: Option<T>,
    pub top_left: Option<T>,
    pub top_right: Option<T>,
    pub top_intersection: Option<T>,

    pub bottom: Option<T>,
    pub bottom_left: Option<T>,
    pub bottom_right: Option<T>,
    pub bottom_intersection: Option<T>,

    pub horizontal: Option<T>,
    pub horizontal_left: Option<T>,
    pub horizontal_right: Option<T>,

    pub vertical_left: Option<T>,
    pub vertical_intersection: Option<T>, // todo: rename to vertical
    pub vertical_right: Option<T>,

    pub intersection: Option<T>,
}

#[derive(Debug, Clone, Default)]
struct BordersMap<T> {
    vertical: HashMap<Position, T>,
    horizontal: HashMap<Position, T>,
    intersection: HashMap<Position, T>,
}

#[derive(Debug, Clone, Default)]
pub struct Line<T> {
    pub horizontal: Option<T>,
    pub intersection: Option<T>,
    pub left: Option<T>,
    pub right: Option<T>,
}

impl<T> Line<T> {
    pub const fn empty() -> Self {
        Self {
            horizontal: None,
            intersection: None,
            left: None,
            right: None,
        }
    }

    pub const fn filled(c: T) -> Self
    where
        T: Copy,
    {
        Self::full(c, c, c, c)
    }

    pub const fn full(horizontal: T, intersection: T, left: T, right: T) -> Self {
        Self {
            horizontal: Some(horizontal),
            intersection: Some(intersection),
            left: Some(left),
            right: Some(right),
        }
    }

    pub const fn short(horizontal: T, intersection: T) -> Self {
        Self {
            horizontal: Some(horizontal),
            intersection: Some(intersection),
            left: None,
            right: None,
        }
    }

    pub fn horizontal(mut self, c: T) -> Self {
        self.horizontal = Some(c);
        self
    }

    pub fn intersection(mut self, c: T) -> Self {
        self.intersection = Some(c);
        self
    }

    pub fn left(mut self, c: T) -> Self {
        self.left = Some(c);
        self
    }

    pub fn right(mut self, c: T) -> Self {
        self.right = Some(c);
        self
    }

    pub fn is_empty(&self) -> bool {
        self.horizontal.is_none()
            && self.horizontal.is_none()
            && self.left.is_none()
            && self.right.is_none()
    }
}

#[derive(Debug, Clone, Default)]
struct BordersLayout {
    horizontal: HashSet<usize>,
    vertical: HashSet<usize>,
    vertical_mid: bool,
    vertical_left: bool,
    vertical_right: bool,
}

pub type Position = (usize, usize);

impl<T: std::fmt::Debug> BordersConfig<T> {
    fn insert_border(&mut self, pos: Position, border: Border<T>) {
        if let Some(c) = border.top {
            self.cells.horizontal.insert(pos, c);
            self.layout.horizontal.insert(pos.0);
        }

        if let Some(c) = border.bottom {
            self.cells.horizontal.insert((pos.0 + 1, pos.1), c);
            self.layout.horizontal.insert(pos.0 + 1);
        }

        if let Some(c) = border.left {
            self.cells.vertical.insert(pos, c);
            self.layout.vertical.insert(pos.1);
        }

        if let Some(c) = border.right {
            self.cells.vertical.insert((pos.0, pos.1 + 1), c);
            self.layout.vertical.insert(pos.1 + 1);
        }

        if let Some(c) = border.left_top_corner {
            self.cells.intersection.insert((pos.0, pos.1), c);
            self.layout.horizontal.insert(pos.0);
            self.layout.vertical.insert(pos.1);
        }

        if let Some(c) = border.right_top_corner {
            self.cells.intersection.insert((pos.0, pos.1 + 1), c);
            self.layout.horizontal.insert(pos.0);
            self.layout.vertical.insert(pos.1 + 1);
        }

        if let Some(c) = border.left_bottom_corner {
            self.cells.intersection.insert((pos.0 + 1, pos.1), c);
            self.layout.horizontal.insert(pos.0 + 1);
            self.layout.vertical.insert(pos.1);
        }

        if let Some(c) = border.right_bottom_corner {
            self.cells.intersection.insert((pos.0 + 1, pos.1 + 1), c);
            self.layout.horizontal.insert(pos.0 + 1);
            self.layout.vertical.insert(pos.1 + 1);
        }
    }

    fn remove_border(&mut self, pos: Position, count_cols: usize) {
        self.cells.horizontal.remove(&pos);
        self.cells.horizontal.remove(&(pos.0 + 1, pos.1));
        self.cells.vertical.remove(&pos);
        self.cells.vertical.remove(&(pos.0, pos.1 + 1));
        self.cells.intersection.remove(&pos);
        self.cells.intersection.remove(&(pos.0 + 1, pos.1));
        self.cells.intersection.remove(&(pos.0, pos.1 + 1));
        self.cells.intersection.remove(&(pos.0 + 1, pos.1 + 1));

        // clean up the layout.

        if !self.check_is_horizontal_set(pos.0) {
            self.layout.horizontal.remove(&pos.0);
        }

        if !self.check_is_horizontal_set(pos.0 + 1) {
            self.layout.horizontal.remove(&(pos.0 + 1));
        }

        if !self.check_is_vertical_set(pos.1, count_cols) {
            self.layout.vertical.remove(&pos.1);
        }

        if !self.check_is_vertical_set(pos.1 + 1, count_cols) {
            self.layout.vertical.remove(&(pos.1 + 1));
        }
    }

    fn get_border(&self, pos: Position, count_rows: usize, count_cols: usize) -> Border<&T> {
        Border {
            top: self.get_horizontal(pos, count_rows),
            bottom: self.get_horizontal((pos.0 + 1, pos.1), count_rows),
            left: self.get_vertical(pos, count_cols),
            left_top_corner: self.get_intersection(pos, count_rows, count_cols),
            left_bottom_corner: self.get_intersection((pos.0 + 1, pos.1), count_rows, count_cols),
            right: self.get_vertical((pos.0, pos.1 + 1), count_cols),
            right_top_corner: self.get_intersection((pos.0, pos.1 + 1), count_rows, count_cols),
            right_bottom_corner: self.get_intersection(
                (pos.0 + 1, pos.1 + 1),
                count_rows,
                count_cols,
            ),
        }
    }

    fn insert_line(&mut self, row: usize, line: Line<T>) {
        if line.is_empty() {
            return;
        }

        if line.left.is_some() {
            self.layout.vertical_left = true;
        }

        if line.right.is_some() {
            self.layout.vertical_right = true;
        }

        if line.intersection.is_some() {
            self.layout.vertical_mid = true;
        }

        self.lines.insert(row, line);
        self.layout.horizontal.insert(row);
    }

    fn set_borders(&mut self, borders: Borders<T>) {
        self.borders = borders;
    }

    fn get_vertical(&self, pos: Position, count_cols: usize) -> Option<&T> {
        self.cells
            .vertical
            .get(&pos)
            .or({
                if pos.1 == count_cols {
                    self.borders.vertical_right.as_ref()
                } else if pos.1 == 0 {
                    self.borders.vertical_left.as_ref()
                } else {
                    self.borders.vertical_intersection.as_ref()
                }
            })
            .or(self.global.as_ref())
    }

    fn get_horizontal(&self, pos: Position, count_rows: usize) -> Option<&T> {
        self.cells
            .horizontal
            .get(&pos)
            .or_else(|| self.lines.get(&pos.0).and_then(|l| l.horizontal.as_ref()))
            .or({
                if pos.0 == 0 {
                    self.borders.top.as_ref()
                } else if pos.0 == count_rows {
                    self.borders.bottom.as_ref()
                } else {
                    self.borders.horizontal.as_ref()
                }
            })
            .or(self.global.as_ref())
    }

    fn get_intersection(&self, pos: Position, count_rows: usize, count_cols: usize) -> Option<&T> {
        let use_top = pos.0 == 0;
        let use_bottom = pos.0 == count_rows;
        let use_left = pos.1 == 0;
        let use_right = pos.1 == count_cols;

        self.cells
            .intersection
            .get(&pos)
            .or_else(|| {
                self.lines.get(&pos.0).and_then(|l| {
                    if use_left && l.left.is_some() {
                        l.left.as_ref()
                    } else if use_right && l.right.is_some() {
                        l.right.as_ref()
                    } else if !use_right && !use_left && l.intersection.is_some() {
                        l.intersection.as_ref()
                    } else {
                        None
                    }
                })
            })
            .or({
                if use_top && use_left {
                    self.borders.top_left.as_ref()
                } else if use_top && use_right {
                    self.borders.top_right.as_ref()
                } else if use_bottom && use_left {
                    self.borders.bottom_left.as_ref()
                } else if use_bottom && use_right {
                    self.borders.bottom_right.as_ref()
                } else if use_top {
                    self.borders.top_intersection.as_ref()
                } else if use_bottom {
                    self.borders.bottom_intersection.as_ref()
                } else if use_left {
                    self.borders.horizontal_left.as_ref()
                } else if use_right {
                    self.borders.horizontal_right.as_ref()
                } else {
                    self.borders.intersection.as_ref()
                }
            })
            .or(self.global.as_ref())
    }

    fn is_line_defined(&self, row: usize) -> bool {
        self.lines.get(&row).map_or(false, |l| {
            l.left.is_some()
                || l.right.is_some()
                || l.intersection.is_some()
                || l.horizontal.is_some()
        })
    }

    fn has_horizontal(&self, row: usize, count_rows: usize) -> bool {
        if self.global.is_some() {
            return true;
        }

        if row == count_rows {
            if self.borders.bottom.is_some()
                || self.borders.bottom_intersection.is_some()
                || self.borders.bottom_left.is_some()
                || self.borders.bottom_right.is_some()
            {
                return true;
            }
        } else if row == 0 {
            if self.borders.top.is_some()
                || self.borders.top_intersection.is_some()
                || self.borders.top_left.is_some()
                || self.borders.top_right.is_some()
            {
                return true;
            }
        } else if self.borders.horizontal.is_some()
            || self.borders.horizontal_left.is_some()
            || self.borders.horizontal_right.is_some()
            || self.borders.intersection.is_some()
        {
            return true;
        }

        if self.is_horizontal_set(row) {
            return true;
        }

        false
    }

    fn has_vertical(&self, col: usize, count_cols: usize) -> bool {
        if self.global.is_some() {
            return true;
        }

        if col == count_cols {
            if self.borders.vertical_right.is_some()
                || self.borders.horizontal_right.is_some()
                || self.borders.top_right.is_some()
                || self.borders.bottom_right.is_some()
            {
                return true;
            }
        } else if col == 0 {
            if self.borders.vertical_left.is_some()
                || self.borders.horizontal_left.is_some()
                || self.borders.top_left.is_some()
                || self.borders.bottom_left.is_some()
            {
                return true;
            }
        } else if self.borders.vertical_intersection.is_some()
            || self.borders.top_intersection.is_some()
            || self.borders.bottom_intersection.is_some()
            || self.borders.intersection.is_some()
        {
            return true;
        }

        if self.is_vertical_set(col, count_cols) {
            return true;
        }

        false
    }

    fn is_horizontal_set(&self, row: usize) -> bool {
        self.layout.horizontal.contains(&row)
    }

    fn is_vertical_set(&self, col: usize, count_cols: usize) -> bool {
        (col == 0 && self.layout.vertical_left)
            || (col == count_cols && self.layout.vertical_right)
            || (col > 0 && col < count_cols && self.layout.vertical_mid)
            || self.layout.vertical.contains(&col)
    }

    fn check_is_horizontal_set(&self, row: usize) -> bool {
        self.is_line_defined(row)
            || self.cells.horizontal.keys().any(|&p| p.0 == row)
            || self.cells.intersection.keys().any(|&p| p.0 == row)
    }

    fn check_is_vertical_set(&self, col: usize, count_cols: usize) -> bool {
        (col == 0 && self.layout.vertical_left)
            || (col == count_cols && self.layout.vertical_right)
            || (col > 0 && col < count_cols && self.layout.vertical_mid)
            || self.cells.vertical.keys().any(|&p| p.1 == col)
            || self.cells.intersection.keys().any(|&p| p.1 == col)
    }
}

fn get_vertical(grid: &Grid, pos: Position) -> Option<&char> {
    let v = grid.borders.get_vertical(pos, grid.count_columns());
    if v.is_some() {
        return v;
    }

    if has_vertical(grid, pos.1) {
        return Some(DEFAULT_BORDER_VERTICAL_SYMBOL_REF);
    }

    None
}

fn get_horizontal(grid: &Grid, pos: Position) -> Option<&char> {
    let v = grid.borders.get_horizontal(pos, grid.count_rows());
    if v.is_some() {
        return v;
    }

    if has_horizontal(grid, pos.0) {
        return Some(DEFAULT_BORDER_HORIZONTAL_SYMBOL_REF);
    }

    None
}

fn get_intersection(grid: &Grid, pos: Position) -> Option<&char> {
    let v = grid
        .borders
        .get_intersection(pos, grid.count_rows(), grid.count_columns());
    if v.is_some() {
        return v;
    }

    if has_horizontal(grid, pos.0) && has_vertical(grid, pos.1) {
        return Some(DEFAULT_BORDER_INTERSECTION_SYMBOL_REF);
    }

    None
}

fn print_grid(
    f: &mut fmt::Formatter<'_>,
    grid: &Grid,
    widths: &[usize],
    mut heights: impl Iterator<Item = usize>,
    cells: &[Vec<CellContent<'_>>],
) -> fmt::Result {
    let table_width = row_width_grid(grid, widths);

    if grid.margin.top.size > 0 {
        let width = table_width + grid.margin.left.size + grid.margin.right.size;
        repeat_lines(f, grid.margin.top.size, width, grid.margin.top.fill)?;
        f.write_char('\n')?;
    }

    #[allow(clippy::needless_range_loop)]
    for row in 0..grid.count_rows() {
        if has_horizontal(grid, row) {
            repeat_char(f, grid.margin.left.fill, grid.margin.left.size)?;
            print_split_line(f, grid, widths, table_width, row)?;
            repeat_char(f, grid.margin.right.fill, grid.margin.right.size)?;
            f.write_char('\n')?;
        }

        let height = heights.next().unwrap();

        let is_last_row = row + 1 == grid.count_rows();

        for i in 0..height {
            repeat_char(f, grid.margin.left.fill, grid.margin.left.size)?;

            for col in 0..grid.count_columns() {
                if is_cell_visible(grid, (row, col)) {
                    let left = get_vertical(grid, (row, col));
                    if let Some(c) = left {
                        #[cfg(feature = "color")]
                        write_colored(f, c, get_vertical_color(grid, (row, col)))?;

                        #[cfg(not(feature = "color"))]
                        c.fmt(f)?;
                    }

                    let style = grid.style(Entity::Cell(row, col));
                    let cell = &cells[row][col];
                    let width = grid_cell_width(grid, widths, (row, col));
                    build_cell_line(f, cell, i, width, height, &style, grid.config.tab_width)?;
                }

                let is_last_column = col + 1 == grid.count_columns();
                if is_last_column {
                    let right = get_vertical(grid, (row, col + 1));
                    if let Some(c) = right {
                        #[cfg(feature = "color")]
                        write_colored(f, c, get_vertical_color(grid, (row, col + 1)))?;

                        #[cfg(not(feature = "color"))]
                        c.fmt(f)?;
                    }
                }
            }

            repeat_char(f, grid.margin.right.fill, grid.margin.right.size)?;

            let is_last_line = i + 1 == height;
            if !(is_last_line && is_last_row) {
                f.write_char('\n')?;
            }
        }
    }

    if has_horizontal(grid, grid.count_rows()) {
        f.write_char('\n')?;
        repeat_char(f, grid.margin.left.fill, grid.margin.left.size)?;
        print_split_line(f, grid, widths, table_width, grid.count_rows())?;
        repeat_char(f, grid.margin.right.fill, grid.margin.right.size)?;
    }

    if grid.margin.bottom.size > 0 {
        f.write_char('\n')?;
        let width = table_width + grid.margin.left.size + grid.margin.right.size;
        repeat_lines(f, grid.margin.bottom.size, width, grid.margin.bottom.fill)?;
    }

    Ok(())
}

fn grid_cell_width(grid: &Grid, widths: &[usize], pos: Position) -> usize {
    let span = grid.get_column_span(pos);
    match span {
        Some(span) => range_width(grid, pos.1, pos.1 + span, widths),
        None => widths[pos.1],
    }
}

fn repeat_lines(f: &mut fmt::Formatter<'_>, size: usize, width: usize, fill: char) -> fmt::Result {
    for i in 0..size {
        repeat_char(f, fill, width)?;

        if i + 1 != size {
            f.write_char('\n')?;
        }
    }

    Ok(())
}

fn print_split_line(
    f: &mut fmt::Formatter<'_>,
    grid: &Grid,
    widths: &[usize],
    max_width: usize,
    row: usize,
) -> fmt::Result {
    let mut char_skip = 0;
    let override_text = grid.override_split_lines.get(&row);
    if let Some(text) = override_text {
        if !text.is_empty() {
            let text = cut_str(text, max_width);
            let line = text.lines().next().unwrap();
            char_skip = string_width(line);
            f.write_str(line)?;
        }
    }

    #[cfg(feature = "color")]
    let mut used_color = None;

    for (col, width) in widths.iter().enumerate() {
        if col == 0 {
            let left = get_intersection(grid, (row, col));
            if let Some(c) = left {
                if char_skip == 0 {
                    #[cfg(feature = "color")]
                    {
                        if let Some(clr) = get_intersection_color(grid, (row, col)) {
                            clr.write_begin_sequence(f)?;
                            used_color = Some(clr);
                        }
                    }

                    c.fmt(f)?;
                } else {
                    char_skip -= 1;
                }
            }
        }

        let mut width = *width;
        if char_skip > 0 {
            let sub = cmp::min(width, char_skip);
            width -= sub;
            char_skip -= sub;
        }

        let main = get_horizontal(grid, (row, col));
        match main {
            Some(c) => {
                #[cfg(feature = "color")]
                {
                    prepare_coloring(f, get_horizontal_color(grid, (row, col)), &mut used_color)?;
                }

                repeat_symbol(f, *c, width)?;
            }
            None => repeat_char(f, DEFAULT_BORDER_HORIZONTAL_CHAR, width)?,
        }

        let right = get_intersection(grid, (row, col + 1));
        if let Some(c) = right {
            if char_skip == 0 {
                #[cfg(feature = "color")]
                {
                    prepare_coloring(
                        f,
                        get_intersection_color(grid, (row, col + 1)),
                        &mut used_color,
                    )?;
                }

                c.fmt(f)?;
            } else {
                char_skip -= 1;
            }
        }
    }

    #[cfg(feature = "color")]
    if let Some(clr) = used_color.take() {
        clr.write_end_sequence(f)?;
    }

    Ok(())
}

#[cfg(feature = "color")]
fn prepare_coloring<'a>(
    f: &mut fmt::Formatter<'_>,
    clr: Option<&'a BorderColor>,
    used_color: &mut Option<&'a BorderColor>,
) -> fmt::Result {
    match clr {
        Some(clr) => match used_color.as_mut() {
            Some(used_clr) => {
                if **used_clr != *clr {
                    used_clr.write_end_sequence(f)?;
                    clr.write_begin_sequence(f)?;
                    *used_clr = clr;
                }
            }
            None => {
                clr.write_begin_sequence(f)?;
                *used_color = Some(clr);
            }
        },
        None => match used_color.take() {
            Some(clr) => clr.write_end_sequence(f)?,
            None => (),
        },
    }

    Ok(())
}

#[cfg(feature = "color")]
fn write_colored(
    f: &mut fmt::Formatter<'_>,
    c: impl fmt::Display,
    clr: Option<&BorderColor>,
) -> fmt::Result {
    if let Some(clr) = &clr {
        clr.write_begin_sequence(f)?;
        c.fmt(f)?;
        clr.write_end_sequence(f)?;
    } else {
        c.fmt(f)?;
    }

    Ok(())
}

#[cfg(feature = "color")]
fn get_intersection_color(grid: &Grid, pos: Position) -> Option<&BorderColor> {
    grid.border_colors
        .get_intersection(pos, grid.count_rows(), grid.count_columns())
}

#[cfg(feature = "color")]
fn get_horizontal_color(grid: &Grid, pos: Position) -> Option<&BorderColor> {
    grid.border_colors.get_horizontal(pos, grid.count_rows())
}

#[cfg(feature = "color")]
fn get_vertical_color(grid: &Grid, pos: Position) -> Option<&BorderColor> {
    grid.border_colors.get_vertical(pos, grid.count_columns())
}

fn row_width_grid(grid: &Grid, widths: &[usize]) -> usize {
    let row_width = widths.iter().sum::<usize>();
    let count_borders = (0..grid.count_columns())
        .filter(|&col| has_vertical(grid, col))
        .count()
        + usize::from(has_vertical(grid, grid.count_columns()));

    row_width + count_borders
}

fn has_vertical(grid: &Grid, col: usize) -> bool {
    grid.borders.has_vertical(col, grid.count_columns())
}

fn has_horizontal(grid: &Grid, row: usize) -> bool {
    grid.borders.has_horizontal(row, grid.count_rows())
}

fn count_tabs(s: &str) -> usize {
    bytecount::count(s.as_bytes(), b'\t')
}

pub fn count_lines(s: &str) -> usize {
    if s.is_empty() {
        return 1;
    }

    bytecount::count(s.as_bytes(), b'\n') + 1
}

fn lookup_entity_value<T>(map: &HashMap<Entity, T>, global: T, entity: Entity) -> T
where
    T: Copy,
{
    match entity {
        Entity::Column(col) => map.get(&Entity::Column(col)).copied(),
        Entity::Row(row) => map.get(&Entity::Row(row)).copied(),
        Entity::Cell(row, col) => map
            .get(&Entity::Cell(row, col))
            .or_else(|| map.get(&Entity::Column(col)))
            .or_else(|| map.get(&Entity::Row(row)))
            .copied(),
        Entity::Global => return global,
    }
    .unwrap_or(global)
}

fn set_entity_value<T>(map: &mut HashMap<Entity, T>, global: &mut T, entity: Entity, value: T) {
    match entity {
        Entity::Global => {
            *global = value;
        }
        _ => {
            map.insert(entity, value);
        }
    }

    invalidate_entity(map, entity);
}

fn invalidate_entity<T>(map: &mut HashMap<Entity, T>, entity: Entity) {
    match entity {
        Entity::Global => {
            map.clear();
        }
        Entity::Column(col) => {
            while let Some(o) = map
                .keys()
                .find(|entity| matches!(entity, Entity::Cell(c, _) if *c == col))
                .copied()
            {
                map.remove(&o);
            }
        }
        Entity::Row(row) => {
            while let Some(o) = map
                .keys()
                .find(|entity| matches!(entity, Entity::Cell(_, r) if *r == row))
                .copied()
            {
                map.remove(&o);
            }
        }
        Entity::Cell(_, _) => (),
    }
}

#[cfg(feature = "color")]
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct BorderColor(String, String);

#[cfg(feature = "color")]
impl BorderColor {
    pub fn new(start: String, end: String) -> Self {
        BorderColor(start, end)
    }

    fn write_begin_sequence(&self, f: &mut impl fmt::Write) -> fmt::Result {
        f.write_str(&self.0)
    }

    fn write_end_sequence(&self, f: &mut impl fmt::Write) -> fmt::Result {
        f.write_str(&self.1)
    }
}

#[cfg(feature = "color")]
impl std::convert::TryFrom<&str> for BorderColor {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match get_ansi_secuences(value) {
            Some((_, start, end)) => Ok(Self::new(start, end)),
            None => Err(()),
        }
    }
}

#[cfg(feature = "color")]
impl std::convert::TryFrom<String> for BorderColor {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

#[cfg(feature = "color")]
#[derive(Debug, Clone, Default)]
pub struct BordersColors {
    pub top: Option<BorderColor>,
    pub top_left: Option<BorderColor>,
    pub top_right: Option<BorderColor>,
    pub top_intersection: Option<BorderColor>,

    pub bottom: Option<BorderColor>,
    pub bottom_left: Option<BorderColor>,
    pub bottom_right: Option<BorderColor>,
    pub bottom_intersection: Option<BorderColor>,

    pub horizontal: Option<BorderColor>,
    pub horizontal_left: Option<BorderColor>,
    pub horizontal_right: Option<BorderColor>,

    pub vertical_left: Option<BorderColor>,
    pub vertical_intersection: Option<BorderColor>,
    pub vertical_right: Option<BorderColor>,

    pub intersection: Option<BorderColor>,
}

#[cfg(feature = "color")]
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Symbol {
    c: char,
    ansi_sequences: Option<BorderColor>,
}

#[cfg(feature = "color")]
impl Symbol {
    const fn new(c: char, ansi_sequences: Option<BorderColor>) -> Self {
        Self { c, ansi_sequences }
    }

    /// Creates a new [`Symbol`] from the String.
    /// The string must contain 1 UTF-8 character and any list of Ansi sequences.
    ///
    /// If it contains more then 1 character `None` will be returned.
    pub fn ansi(s: impl AsRef<str>) -> Option<Self> {
        let s = s.as_ref();

        let mut chars = s.chars();
        let c = chars.next();
        let no_other_chars = chars.next().is_none();
        drop(chars);
        match c {
            Some(c) if no_other_chars => return Some(Self::new(c, None)),
            _ => (),
        }

        if string_width(s) != 1 {
            return None;
        }

        let (c, start, end) = get_ansi_secuences(s)?;
        if start.is_empty() && end.is_empty() {
            return Some(Self::new(c, None));
        }

        Some(Self::new(c, Some(BorderColor(start, end))))
    }

    /// A function which create a [`Symbol`] from [`char`].
    pub const fn from_char(c: char) -> Self {
        Self::new(c, None)
    }

    /// A function which returns a used [`char`].
    pub const fn c(&self) -> char {
        self.c
    }

    /// A function which returns a used [`char`].
    pub fn color(self) -> Option<BorderColor> {
        self.ansi_sequences
    }
}

#[cfg(feature = "color")]
impl Default for Symbol {
    fn default() -> Self {
        Self::from_char(char::default())
    }
}

#[cfg(feature = "color")]
impl From<char> for Symbol {
    fn from(c: char) -> Self {
        Self::from_char(c)
    }
}

#[cfg(feature = "color")]
fn get_ansi_secuences(s: &str) -> Option<(char, String, String)> {
    let mut original = ansi_str::get_blocks(s);
    let block = original.next()?;

    let c = block.text().chars().next()?;

    let start = block.start().to_string();
    let end = block.end().to_string();

    Some((c, start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replace_tab_test() {
        assert_eq!(replace_tab("123\t\tabc\t", 3), "123      abc   ");

        assert_eq!(replace_tab("\t", 0), "");
        assert_eq!(replace_tab("\t", 3), "   ");
        assert_eq!(replace_tab("123\tabc", 3), "123   abc");
        assert_eq!(replace_tab("123\tabc\tzxc", 0), "123abczxc");

        assert_eq!(replace_tab("\\t", 0), "\\t");
        assert_eq!(replace_tab("\\t", 4), "\\t");
        assert_eq!(replace_tab("123\\tabc", 0), "123\\tabc");
        assert_eq!(replace_tab("123\\tabc", 4), "123\\tabc");
    }

    #[test]
    fn string_width_emojie_test() {
        // ...emojis such as “joy”, which normally take up two columns when printed in a terminal
        // https://github.com/mgeisler/textwrap/pull/276
        assert_eq!(string_width("🎩"), 2);
        assert_eq!(string_width("Rust 💕"), 7);
        assert_eq!(string_width_multiline("Go 👍\nC 😎"), 5);
    }

    #[test]
    fn horizontal_aligment_test() {
        use std::fmt;

        struct F<'a>(&'a str, AlignmentHorizontal, usize);

        impl fmt::Display for F<'_> {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let width = string_width(self.0);
                print_text_formated(f, self.0, width, self.1, self.2, 0)
            }
        }

        assert_eq!(F("AAA", AlignmentHorizontal::Right, 4).to_string(), " AAA");
        assert_eq!(F("AAA", AlignmentHorizontal::Left, 4).to_string(), "AAA ");
        assert_eq!(F("AAA", AlignmentHorizontal::Center, 4).to_string(), "AAA ");
        assert_eq!(F("🎩", AlignmentHorizontal::Center, 4).to_string(), " 🎩 ");
        assert_eq!(F("🎩", AlignmentHorizontal::Center, 3).to_string(), "🎩 ");

        #[cfg(feature = "color")]
        {
            use owo_colors::OwoColorize;
            let text = "Colored Text".red().to_string();
            assert_eq!(
                F(&text, AlignmentHorizontal::Center, 15).to_string(),
                format!(" {}  ", text)
            );
        }
    }

    #[test]
    fn vertical_aligment_test() {
        use AlignmentVertical::*;

        assert_eq!(indent_from_top(Bottom, 1, 1), 0);
        assert_eq!(indent_from_top(Top, 1, 1), 0);
        assert_eq!(indent_from_top(Center, 1, 1), 0);
        assert_eq!(indent_from_top(Bottom, 3, 1), 2);
        assert_eq!(indent_from_top(Top, 3, 1), 0);
        assert_eq!(indent_from_top(Center, 3, 1), 1);
        assert_eq!(indent_from_top(Center, 4, 1), 1);
    }

    #[cfg(feature = "color")]
    #[test]
    fn colored_string_width_test() {
        use owo_colors::OwoColorize;
        assert_eq!(string_width(&"hello world".red().to_string()), 11);
        assert_eq!(
            string_width_multiline(&"hello\nworld".blue().to_string()),
            5
        );
        assert_eq!(string_width("\u{1b}[34m0\u{1b}[0m"), 1);
        assert_eq!(string_width(&"0".red().to_string()), 1);
    }

    #[test]
    fn strip_test() {
        assert_eq!(cut_str("123456", 0), "");
        assert_eq!(cut_str("123456", 3), "123");
        assert_eq!(cut_str("123456", 10), "123456");

        assert_eq!(cut_str("a week ago", 4), "a we");

        assert_eq!(cut_str("😳😳😳😳😳", 0), "");
        assert_eq!(cut_str("😳😳😳😳😳", 3), "😳�");
        assert_eq!(cut_str("😳😳😳😳😳", 4), "😳😳");
        assert_eq!(cut_str("😳😳😳😳😳", 20), "😳😳😳😳😳");

        assert_eq!(cut_str("🏳️🏳️", 0), "");
        assert_eq!(cut_str("🏳️🏳️", 1), "🏳");
        assert_eq!(cut_str("🏳️🏳️", 2), "🏳\u{fe0f}🏳");
        assert_eq!(string_width("🏳️🏳️"), string_width("🏳\u{fe0f}🏳"));

        assert_eq!(cut_str("🎓", 1), "�");
        assert_eq!(cut_str("🎓", 2), "🎓");

        assert_eq!(cut_str("🥿", 1), "�");
        assert_eq!(cut_str("🥿", 2), "🥿");

        assert_eq!(cut_str("🩰", 1), "�");
        assert_eq!(cut_str("🩰", 2), "🩰");

        assert_eq!(cut_str("👍🏿", 1), "�");
        assert_eq!(cut_str("👍🏿", 2), "👍");
        assert_eq!(cut_str("👍🏿", 3), "👍�");
        assert_eq!(cut_str("👍🏿", 4), "👍🏿");

        assert_eq!(cut_str("🇻🇬", 1), "🇻");
        assert_eq!(cut_str("🇻🇬", 2), "🇻🇬");
        assert_eq!(cut_str("🇻🇬", 3), "🇻🇬");
        assert_eq!(cut_str("🇻🇬", 4), "🇻🇬");
    }

    #[cfg(feature = "color")]
    #[test]
    fn strip_color_test() {
        use owo_colors::OwoColorize;

        let numbers = "123456".red().on_bright_black().to_string();

        assert_eq!(cut_str(&numbers, 0), "\u{1b}[31;100m\u{1b}[39m\u{1b}[49m");
        assert_eq!(
            cut_str(&numbers, 3),
            "\u{1b}[31;100m123\u{1b}[39m\u{1b}[49m"
        );
        assert_eq!(cut_str(&numbers, 10), "\u{1b}[31;100m123456\u{1b}[0m");

        let emojies = "😳😳😳😳😳".red().on_bright_black().to_string();

        assert_eq!(cut_str(&emojies, 0), "\u{1b}[31;100m\u{1b}[39m\u{1b}[49m");
        assert_eq!(
            cut_str(&emojies, 3),
            "\u{1b}[31;100m😳\u{1b}[39m\u{1b}[49m�"
        );
        assert_eq!(
            cut_str(&emojies, 4),
            "\u{1b}[31;100m😳😳\u{1b}[39m\u{1b}[49m"
        );
        assert_eq!(cut_str(&emojies, 20), "\u{1b}[31;100m😳😳😳😳😳\u{1b}[0m");

        let emojies = "🏳️🏳️".red().on_bright_black().to_string();

        assert_eq!(cut_str(&emojies, 0), "\u{1b}[31;100m\u{1b}[39m\u{1b}[49m");
        assert_eq!(cut_str(&emojies, 1), "\u{1b}[31;100m🏳\u{1b}[39m\u{1b}[49m");
        assert_eq!(
            cut_str(&emojies, 2),
            "\u{1b}[31;100m🏳\u{fe0f}🏳\u{1b}[39m\u{1b}[49m"
        );
        assert_eq!(
            string_width(&emojies),
            string_width("\u{1b}[31;100m🏳\u{fe0f}🏳\u{1b}[39m\u{1b}[49m")
        );
    }

    #[test]
    fn count_lines_test() {
        assert_eq!(
            count_lines("\u{1b}[37mnow is the time for all good men\n\u{1b}[0m"),
            2
        );
        assert_eq!(count_lines("now is the time for all good men\n"), 2);
    }
}
