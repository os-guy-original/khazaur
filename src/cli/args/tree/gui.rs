use crate::error::Result;
use crate::cli::args::tree::data::get_flat_tree;
use gtk4::prelude::*;
use gtk4::{
    Application, ApplicationWindow, Orientation, TreeStore, TreeView, TreeViewColumn, 
    CellRendererText, SearchEntry, ScrolledWindow, PolicyType, 
    Picture, Label, HeaderBar
};
use gtk4::gdk;
use gtk4::gio;
use gtk4::glib;

// Embed the logo
const LOGO_BYTES: &[u8] = include_bytes!("../../../../res/logo/khazaur.svg");

pub fn run(package: &str) -> Result<()> {
    // Initialize GTK
    let app_id = format!("org.khazaur.tree.{}", package);
    let app = Application::builder()
        .application_id(app_id)
        .build();

    let package_clone = package.to_string();
    app.connect_activate(move |app| {
        build_ui(app, &package_clone);
    });

    app.run_with_args(&Vec::<String>::new());
    Ok(())
}

fn build_ui(app: &Application, package: &str) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title(format!("Khazaur - {}", package))
        .default_width(700)
        .default_height(600)
        .build();

    // Use native HeaderBar for better system integration and branding
    let header_bar = HeaderBar::new();
    window.set_titlebar(Some(&header_bar));

    // Logo
    let bytes = glib::Bytes::from_static(LOGO_BYTES);
    let stream = gio::MemoryInputStream::from_bytes(&bytes);
    let logo_widget = match gtk4::gdk_pixbuf::Pixbuf::from_stream_at_scale(&stream, 32, 32, true, None::<&gio::Cancellable>) {
        Ok(pixbuf) => {
            let texture = gdk::Texture::for_pixbuf(&pixbuf);
            Picture::for_paintable(&texture)
        },
        Err(e) => {
            eprintln!("Failed to load logo: {}", e);
            Picture::new()
        }
    };
    logo_widget.set_margin_end(10);
    
    // Title Box (Logo + text)
    let title_box = gtk4::Box::new(Orientation::Horizontal, 0);
    title_box.append(&logo_widget);
    
    let title_label = Label::new(Some("Khazaur"));
    title_label.set_css_classes(&["title"]); // Use standard title style if available, or just bold
    // Actually HeaderBar handles title automatically if we don't set custom title widget.
    // But we want Logo + Text.
    // Let's pack them.
    title_box.append(&title_label);
    
    header_bar.pack_start(&title_box);
    
    // Search Entry
    let search_entry = SearchEntry::new();
    search_entry.set_placeholder_text(Some("Search dependencies..."));
    search_entry.set_width_request(250);
    header_bar.pack_end(&search_entry);

    // Tree Area
    let store = TreeStore::new(&[String::static_type()]);

    let data = get_flat_tree(package).unwrap_or_default();
    
    // Populate tree store logic
    store.clear();
    let mut iter_stack: Vec<(usize, gtk4::TreeIter)> = Vec::new();
    
    for (depth, name) in data {
        // While stack top depth >= current depth, pop (go up)
        while let Some((d, _)) = iter_stack.last() {
             if *d >= depth {
                 iter_stack.pop();
             } else {
                 break;
             }
        }
        
        let parent = iter_stack.last().map(|(_, iter)| iter);
        let new_iter = store.insert_with_values(parent, None, &[(0, &name)]);
        
        iter_stack.push((depth, new_iter));
    }

    // Tree View
    let tree_view = TreeView::new();
    tree_view.set_model(Some(&store));
    tree_view.set_headers_visible(false);
    tree_view.set_enable_search(true);
    tree_view.set_search_entry(Some(&search_entry));

    // Column
    let column = TreeViewColumn::new();
    let cell = CellRendererText::new();
    column.pack_start(&cell, true);
    column.add_attribute(&cell, "text", 0);
    tree_view.append_column(&column);
    
    tree_view.expand_all();

    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Automatic)
        .vscrollbar_policy(PolicyType::Automatic)
        .child(&tree_view)
        .vexpand(true)
        .build();
        
    window.set_child(Some(&scrolled));

    window.present();
}
