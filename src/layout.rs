
use std::cell;

use gtk;

const SPACING: i32 = 5;

pub fn grid(rows: &[&Fn(GridRow) -> GridRow]) -> gtk::Grid {
    let grid = gtk::Grid::new();
    for index in 0..rows.len() {
        rows[index](GridRow {
            grid: grid.clone(),
            row: index as i32,
            column: cell::Cell::new(0),
        });
    }
    grid
}

pub struct GridRow {
    grid: gtk::Grid,
    row: i32,
    column: cell::Cell<i32>,
}

impl GridRow {

    pub fn add_column<W>(self, widget: &W) -> Self where W: gtk::IsA<gtk::Widget> {
        use gtk::prelude::*;

        self.grid.attach(widget, self.column.get(), self.row, 1, 1);
        self.column.set(self.column.get() + 1);
        self
    }
}

pub fn vbox() -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Vertical, SPACING)
}

pub fn hbox() -> gtk::Box {
    gtk::Box::new(gtk::Orientation::Horizontal, SPACING)
}

pub trait BuildBox {

    type Output;

    fn add_start<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget>;

    fn add_start_fill<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget>;

    fn add_end<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget>;

    fn add_end_fill<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget>;
}

impl BuildBox for gtk::Box {

    type Output = gtk::Box;

    fn add_start<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget> {
        use gtk::prelude::*;
        self.pack_start(child, false, true, 0);
        self
    }

    fn add_start_fill<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget> {
        use gtk::prelude::*;
        self.pack_start(child, true, true, 0);
        self
    }

    fn add_end<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget> {
        use gtk::prelude::*;
        self.pack_end(child, false, true, 0);
        self
    }

    fn add_end_fill<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget> {
        use gtk::prelude::*;
        self.pack_end(child, true, true, 0);
        self
    }
}

impl<'a> BuildBox for &'a gtk::Box {

    type Output = ();

    fn add_start<W>(self, child: &W) where W: gtk::IsA<gtk::Widget> {
        use gtk::prelude::*;
        self.pack_start(child, false, true, 0);
    }

    fn add_start_fill<W>(self, child: &W) where W: gtk::IsA<gtk::Widget> {
        use gtk::prelude::*;
        self.pack_start(child, true, true, 0);
    }

    fn add_end<W>(self, child: &W) where W: gtk::IsA<gtk::Widget> {
        use gtk::prelude::*;
        self.pack_end(child, false, true, 0);
    }

    fn add_end_fill<W>(self, child: &W) where W: gtk::IsA<gtk::Widget> {
        use gtk::prelude::*;
        self.pack_end(child, true, true, 0);
    }
}

pub fn hpaned(position: Option<i32>) -> gtk::Paned {
    use gtk::prelude::*;

    let paned = gtk::Paned::new(gtk::Orientation::Horizontal);
    if let Some(position) = position {
        paned.set_position(position);
    }
    paned
}

pub fn vpaned(position: Option<i32>) -> gtk::Paned {
    use gtk::prelude::*;

    let paned = gtk::Paned::new(gtk::Orientation::Vertical);
    if let Some(position) = position {
        paned.set_position(position);
    }
    paned
}

pub trait BuildPaned {

    type Output;

    fn add1_primary<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget>;

    fn add1_secondary<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget>;

    fn add2_primary<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget>;

    fn add2_secondary<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget>;
}

impl BuildPaned for gtk::Paned {

    type Output = gtk::Paned;

    fn add1_primary<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget> {
        use gtk::prelude::*;
        self.pack1(child, true, true);
        self
    }

    fn add1_secondary<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget> {
        use gtk::prelude::*;
        self.pack1(child, false, true);
        self
    }

    fn add2_primary<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget> {
        use gtk::prelude::*;
        self.pack2(child, true, true);
        self
    }

    fn add2_secondary<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget> {
        use gtk::prelude::*;
        self.pack2(child, false, true);
        self
    }
}

impl<'a> BuildPaned for &'a gtk::Paned {

    type Output = ();

    fn add1_primary<W>(self, child: &W) where W: gtk::IsA<gtk::Widget> {
        use gtk::prelude::*;
        self.pack1(child, true, true);
    }

    fn add1_secondary<W>(self, child: &W) where W: gtk::IsA<gtk::Widget> {
        use gtk::prelude::*;
        self.pack1(child, false, true);
    }

    fn add2_primary<W>(self, child: &W) where W: gtk::IsA<gtk::Widget> {
        use gtk::prelude::*;
        self.pack2(child, true, true);
    }

    fn add2_secondary<W>(self, child: &W) where W: gtk::IsA<gtk::Widget> {
        use gtk::prelude::*;
        self.pack2(child, false, true);
    }
}
