use crate::error::Result;
use crate::cli::args::tree::data::get_flat_tree;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Span, Line},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Terminal,
};
use std::{error::Error, io};

struct App {
    items: Vec<(usize, String)>, // (depth, name)
    // Actually, easier: visibility mask?
    // Let's keep it simple: Expand/Collapse All or just navigation.
    // User asked for "expandable/collapsible stuff".
    // We need to track which nodes are collapsed.
    // A set of indices that are collapsed.
    collapsed: std::collections::HashSet<usize>,
    state: ListState,
    search_query: String,
    search_mode: bool,
}

impl App {
    fn new(items: Vec<(usize, String)>) -> App {
        let mut state = ListState::default();
        state.select(Some(0));
        App {
            items,
            collapsed: std::collections::HashSet::new(),
            state,
            search_query: String::new(),
            search_mode: false,
        }
    }

    fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                // Find next visible item
                let mut next_i = i + 1;
                while next_i < self.items.len() {
                    if self.is_visible(next_i) {
                        break;
                    }
                    next_i += 1;
                }
                if next_i < self.items.len() {
                    next_i
                } else {
                    i // Stay at end
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }

    fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                let mut prev_i = if i == 0 { 0 } else { i - 1 };
                // Backtrack to find nearest visible
                while prev_i > 0 && !self.is_visible(prev_i) {
                     prev_i -= 1;
                }
                // Double check 0
                 if !self.is_visible(prev_i) { i } else { prev_i }
            }
            None => 0,
        };
        self.state.select(Some(i));
    }



    fn is_visible(&self, index: usize) -> bool {
        // an item is visible if none of its parents are collapsed.
        // We need to scan backwards to find parents.
        if index == 0 { return true; }
        
        let (my_depth, _) = self.items[index];
        let mut i = index;
        
        // Look backwards
        while i > 0 {
            i -= 1;
            let (parent_depth, _) = self.items[i];
            
            if parent_depth < my_depth {
                // This is a direct or indirect parent
                // If it is collapsed, I am hidden
                if self.collapsed.contains(&i) {
                    return false;
                }
                
                // If we reached depth 0, we are done checking parents?
                // No, we need to check ALL ancestors.
                // But this loop naturally finds the nearest parent, then that parent's parent, etc (as long as depths strictly decrease).
                // Actually, logic is: Find immediate parent. If collapsed -> hidden. Else -> repeat for parent.
                
                // Optimization: Just check if *any* ancestor with depth < my_depth is in collapsed set?
                // Not quite, it must be a *direct line* ancestor.
                
                // Correct logic:
                // Scan backwards. Record the running minimum depth seen.
                // If we see a node with depth < running_min, checks it.
                // If that node is collapsed, return false.
                // Stop when we reach depth 0 or index 0.
            }
        }
        
        // Re-implement correctly:
        // An item is visible if all its ancestors are expanded.
        // Ancestors are nodes appearing before it with strictly lower depth, 
        // such that no node between ancestor and item has depth <= ancestor.depth.
        
        let mut current_idx = index;
        let mut current_depth = self.items[index].0;
        
        while current_idx > 0 {
             current_idx -= 1;
             let (d, _) = self.items[current_idx];
             if d < current_depth {
                 // This is an ancestor
                 if self.collapsed.contains(&current_idx) {
                     return false;
                 }
                 current_depth = d; // Move up to finding this ancestor's parent
             }
        }
        
        true
    }
}

pub fn run(package: &str) -> Result<()> {
    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app_result = run_app(&mut terminal, package);

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = app_result {
        println!("{:?}", err);
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, package: &str) -> std::result::Result<(), Box<dyn Error>> {
    let items = get_flat_tree(package).map_err(|e| Box::new(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
    let mut app = App::new(items);

    loop {
        terminal.draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(3), // Search / Header
                        Constraint::Min(0),    // Tree
                        Constraint::Length(3), // Help
                    ]
                    .as_ref(),
                )
                .split(f.area());

            // Header / Search
            let header_text = if app.search_mode {
                format!("Search: {}_", app.search_query)
            } else {
                format!("Tree for: {} (Press '/' to search)", package)
            };
            
            let header = Paragraph::new(header_text)
                .style(Style::default().fg(if app.search_mode { Color::Yellow } else { Color::Cyan }))
                .block(Block::default().borders(Borders::ALL));
            f.render_widget(header, chunks[0]);

            // Tree items
            // Filter/Collect visible items
            // We need to map original indices to display list items?
            // No, List widget takes items.
            // But we need to keep track of indices for scrolling.
            
            // This is tricky: ListState assumes contiguous indices 0..N.
            // If we hide items, the list shrinks.
            // So app.state index refers to the *filtered* list.
            // BUT our "collapsed" set refers to *original* indices.
            // So we need to map: DisplayIndex -> OriginalIndex.
            
            let mut display_items = Vec::new();
            let mut visible_indices = Vec::new(); // map display_idx -> original_idx
            
            for (idx, (depth, name)) in app.items.iter().enumerate() {
                if app.is_visible(idx) {
                    
                    // Apply search filter if not empty
                    if !app.search_query.is_empty() && !name.to_lowercase().contains(&app.search_query.to_lowercase()) {
                        // If it doesn't match, maybe we still show it if a child matches?
                        // Simple search: just filter matching nodes? That breaks the tree structure visually.
                        // Better search: Highlight matches.
                    }
                    
                    visible_indices.push(idx);
                    
                    let prefix = "  ".repeat(*depth);
                    let symbol = if app.collapsed.contains(&idx) { "▶ " } else { "▼ " };
                    let leaf_symbol = "• "; // For leaves?
                    
                    // Check if it's a leaf (next item has <= depth)
                    let is_leaf = if idx + 1 < app.items.len() {
                        app.items[idx + 1].0 <= *depth
                    } else {
                        true
                    };
                    
                    let marker = if is_leaf { leaf_symbol } else { symbol };
                    
                    let style = if !app.search_query.is_empty() && name.to_lowercase().contains(&app.search_query.to_lowercase()) {
                         Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                    } else {
                         Style::default()
                    };

                    display_items.push(ListItem::new(Line::from(vec![
                        Span::raw(prefix),
                        Span::styled(marker, if is_leaf { Style::default().fg(Color::DarkGray) } else { Style::default().fg(Color::Green) }),
                        Span::styled(name.clone(), style),
                    ])));
                }
            }

            let list = List::new(display_items)
                .block(Block::default().borders(Borders::ALL).title("Dependencies"))
                .highlight_style(Style::default().add_modifier(Modifier::BOLD).fg(Color::Magenta))
                .highlight_symbol(">> ");
                
            f.render_stateful_widget(list, chunks[1], &mut app.state);
            
            // Help
            let help = Paragraph::new("q: Quit | j/k: Nav | Enter/Space: Toggle | /: Search")
                .style(Style::default().fg(Color::Gray));
            f.render_widget(help, chunks[2]);
            
            // Handle mapping logic for toggling
            // WHEN USER PRESSES ENTER -> we need to handle it in event loop below, but we need visible_indices context.
            // But we can't access visible_indices in event loop easily unless we recompute it.
            // The logic in event loop recomputes it.
            
        })?;

        if let Event::Key(key) = event::read()? {
            if app.search_mode {
                 match key.code {
                     KeyCode::Enter => {
                         app.search_mode = false;
                     }
                     KeyCode::Esc => {
                         app.search_mode = false;
                         app.search_query.clear();
                     }
                     KeyCode::Backspace => {
                         app.search_query.pop();
                     }
                     KeyCode::Char(c) => {
                         app.search_query.push(c);
                     }
                     _ => {}
                 }
            } else {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('j') | KeyCode::Down => app.next(),
                    KeyCode::Char('k') | KeyCode::Up => app.previous(),
                    KeyCode::Char('/') => {
                        app.search_mode = true;
                        app.search_query.clear();
                    }
                    KeyCode::Enter | KeyCode::Char(' ') => {
                        // Calculate visible indices again to map selection
                        let mut visible_indices = Vec::new();
                        for (idx, _) in app.items.iter().enumerate() {
                            if app.is_visible(idx) {
                                visible_indices.push(idx);
                            }
                        }
                        
                        // We need to translate display selection to original item index
                        if let Some(selected_display_idx) = app.state.selected() {
                            if selected_display_idx < visible_indices.len() {
                                let original_idx = visible_indices[selected_display_idx];
                                
                                // Actually calling toggle_collapse works on app.state.selected(), 
                                // BUT toggle_collapse assumes state.selected IS the index? 
                                // NO, my toggle_collapse implementation used state.selected() as index into ITEMS?
                                // CHECK toggle_collapse implementation:
                                // "if let Some(selected) = self.state.selected() ... if self.collapsed.contains(&selected)"
                                // THIS IS WRONG. self.state.selected() is the filtered index.
                                // self.collapsed stores ORIGINAL indices.
                                
                                // So I must fix toggle_collapse to take original index or implement mapping there.
                                // Or better: perform logic here.
                                
                                if app.collapsed.contains(&original_idx) {
                                     app.collapsed.remove(&original_idx);
                                } else {
                                     // Only collapse if not leaf
                                    if original_idx + 1 < app.items.len() && app.items[original_idx + 1].0 > app.items[original_idx].0 {
                                         app.collapsed.insert(original_idx);
                                    }
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
