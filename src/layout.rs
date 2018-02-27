
use gtk;

const SPACING: i32 = 5;

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
        use gtk::{ BoxExt };
        self.pack_start(child, false, true, 0);
        self
    }

    fn add_start_fill<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget> {
        use gtk::{ BoxExt };
        self.pack_start(child, true, true, 0);
        self
    }

    fn add_end<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget> {
        use gtk::{ BoxExt };
        self.pack_end(child, false, true, 0);
        self
    }

    fn add_end_fill<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget> {
        use gtk::{ BoxExt };
        self.pack_end(child, true, true, 0);
        self
    }
}

impl<'a> BuildBox for &'a gtk::Box {

    type Output = ();

    fn add_start<W>(self, child: &W) where W: gtk::IsA<gtk::Widget> {
        use gtk::{ BoxExt };
        self.pack_start(child, false, true, 0);
    }

    fn add_start_fill<W>(self, child: &W) where W: gtk::IsA<gtk::Widget> {
        use gtk::{ BoxExt };
        self.pack_start(child, true, true, 0);
    }

    fn add_end<W>(self, child: &W) where W: gtk::IsA<gtk::Widget> {
        use gtk::{ BoxExt };
        self.pack_end(child, false, true, 0);
    }

    fn add_end_fill<W>(self, child: &W) where W: gtk::IsA<gtk::Widget> {
        use gtk::{ BoxExt };
        self.pack_end(child, true, true, 0);
    }
}

pub fn hpaned(position: Option<i32>) -> gtk::Paned {
    use gtk::{ PanedExt };

    let paned = gtk::Paned::new(gtk::Orientation::Horizontal);
    if let Some(position) = position {
        paned.set_position(position);
    }
    paned
}

pub fn vpaned(position: Option<i32>) -> gtk::Paned {
    use gtk::{ PanedExt };

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
        use gtk::{ PanedExt };
        self.pack1(child, true, true);
        self
    }

    fn add1_secondary<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget> {
        use gtk::{ PanedExt };
        self.pack1(child, false, true);
        self
    }

    fn add2_primary<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget> {
        use gtk::{ PanedExt };
        self.pack2(child, true, true);
        self
    }

    fn add2_secondary<W>(self, child: &W) -> Self::Output where W: gtk::IsA<gtk::Widget> {
        use gtk::{ PanedExt };
        self.pack2(child, false, true);
        self
    }
}

impl<'a> BuildPaned for &'a gtk::Paned {

    type Output = ();

    fn add1_primary<W>(self, child: &W) where W: gtk::IsA<gtk::Widget> {
        use gtk::{ PanedExt };
        self.pack1(child, true, true);
    }

    fn add1_secondary<W>(self, child: &W) where W: gtk::IsA<gtk::Widget> {
        use gtk::{ PanedExt };
        self.pack1(child, false, true);
    }

    fn add2_primary<W>(self, child: &W) where W: gtk::IsA<gtk::Widget> {
        use gtk::{ PanedExt };
        self.pack2(child, true, true);
    }

    fn add2_secondary<W>(self, child: &W) where W: gtk::IsA<gtk::Widget> {
        use gtk::{ PanedExt };
        self.pack2(child, false, true);
    }
}

/*

pub fn vbox() -> BoxBuilder {
    BoxBuilder {
        widget: gtk::Box::new(gtk::Orientation::Vertical, SPACING),
    }
}

pub fn hbox() -> BoxBuilder {
    BoxBuilder {
        widget: gtk::Box::new(gtk::Orientation::Horizontal, SPACING),
    }
}

pub struct BoxBuilder {
    widget: gtk::Box,
}

impl BoxBuilder {

    pub fn pack_start<W>(self, widget: &W) -> Self where W: gtk::IsA<gtk::Widget> {
        use gtk::{ BoxExt };

        self.widget.pack_start(widget, false, true, 0);
        self
    }

    pub fn pack_start_fill<W>(self, widget: &W) -> Self where W: gtk::IsA<gtk::Widget> {
        use gtk::{ BoxExt };

        self.widget.pack_start(widget, true, true, 0);
        self
    }

    pub fn into_widget(self) -> gtk::Box { self.widget }
}

pub fn vpaned(position: Option<i32>) -> PanedBuilder {
    PanedBuilder {
        widget: gtk::Paned::new(gtk::Orientation::Vertical),
    }
}

pub fn hpaned(position: Option<i32>) -> PanedBuilder {
    PanedBuilder {
        widget: gtk::Paned::new(gtk::Orientation::Horizontal),
    }
}

pub struct PanedBuilder {
    widget: gtk::Paned,
}

impl PanedBuilder {

    pub fn pack<W>(self, widget: &W) -> PanedBuilderOne where W: gtk::IsA<gtk::Widget> {
        use gtk::{ PanedExt };

        self.widget.pack1(widget, false, true);
        PanedBuilderOne { widget: self.widget }
    }

    pub fn pack_primary<W>(self, widget: &W) -> PanedBuilderOne where W: gtk::IsA<gtk::Widget> {
        use gtk::{ PanedExt };

        self.widget.pack1(widget, true, true);
        PanedBuilderOne { widget: self.widget }
    }
}

pub struct PanedBuilderOne {
    widget: gtk::Paned,
}

impl PanedBuilderOne {

    pub fn pack<W>(self, widget: &W) -> gtk::Paned where W: gtk::IsA<gtk::Widget> {
        use gtk::{ PanedExt };

        self.widget.pack2(widget, false, true);
        self.widget
    }

    pub fn pack_primary<W>(self, widget: &W) -> gtk::Paned where W: gtk::IsA<gtk::Widget> {
        use gtk::{ PanedExt };

        self.widget.pack2(widget, true, true);
        self.widget
    }
}
*/
