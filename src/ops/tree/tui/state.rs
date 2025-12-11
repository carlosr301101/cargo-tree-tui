use std::path::PathBuf;

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent};

use crate::core::DependencyTree;

use super::widget::TreeWidgetState;

#[derive(Debug)]
pub struct TuiState {
    pub running: bool,
    pub dependency_tree: DependencyTree,
    pub tree_widget_state: TreeWidgetState,
    pub show_help: bool,
    pub search_active: bool,
    pub search_query: String,
    pub search_results: Vec<crate::core::NodeId>,
    pub search_result_index: Option<usize>,
}

impl TuiState {
    pub fn new(manifest_path: Option<PathBuf>) -> Result<Self> {
        let dependency_tree = DependencyTree::load(manifest_path)?;
        let mut tree_widget_state = TreeWidgetState::default();
        tree_widget_state.expand_all(&dependency_tree);
        Ok(TuiState {
            running: true,
            dependency_tree,
            tree_widget_state,
            show_help: false,
            search_active: false,
            search_query: String::new(),
            search_results: Vec::new(),
            search_result_index: None,
        })
    }

    pub fn handle_key_event(&mut self, key_event: KeyEvent) {
        if self.show_help {
            // Close help popup on any key press
            self.show_help = false;
        }

        // Handle search mode separately
        if self.search_active {
            match (key_event.code, key_event.modifiers) {
                (KeyCode::Enter, _) => {
                    // Exit search mode but keep highlights
                    self.search_active = false;
                }
                (KeyCode::Esc, _) => {
                    // Cancel search and clear everything
                    self.clear_search();
                }
                (KeyCode::Backspace, _) => {
                    // Remove last character from search query
                    self.search_query.pop();
                    self.perform_search();
                }
                (KeyCode::Char(c), _) if c != '/' => {
                    // Add character to search query
                    self.search_query.push(c);
                    self.perform_search();
                }
                (KeyCode::Down, _) => {
                    // Navigate to next search result
                    self.next_search_result();
                }
                (KeyCode::Up, _) => {
                    // Navigate to previous search result
                    self.prev_search_result();
                }
                _ => {}
            }
        } else {
            match (key_event.code, key_event.modifiers) {
                (KeyCode::Char('/'), _) => {
                    // Start search mode
                    self.search_active = true;
                    self.search_query.clear();
                    self.search_result_index = None;
                }
                (KeyCode::Char('n'), _) => {
                    // Go to next search result
                    self.next_search_result();
                }
                (KeyCode::Char('N'), _) => {
                    // Go to previous search result (shift+n)
                    self.prev_search_result();
                }
                (KeyCode::Char('c'), _) => {
                    // Clear search highlights
                    self.clear_search();
                }
                (KeyCode::Char('q'), _) => {
                    self.running = false;
                }
                (KeyCode::Char('?'), _) => {
                    self.show_help = !self.show_help;
                }
                (KeyCode::Char('p'), _) => {
                    self.tree_widget_state.select_parent(&self.dependency_tree);
                }
                (KeyCode::Char(']'), _) => {
                    self.tree_widget_state
                        .select_next_sibling(&self.dependency_tree);
                }
                (KeyCode::Char('['), _) => {
                    self.tree_widget_state
                        .select_previous_sibling(&self.dependency_tree);
                }
                (KeyCode::Down, _) => {
                    if self.search_active {
                        self.next_search_result();
                    } else {
                        self.tree_widget_state.select_next(&self.dependency_tree);
                    }
                }
                (KeyCode::Up, _) => {
                    if self.search_active {
                        self.prev_search_result();
                    } else {
                        self.tree_widget_state
                            .select_previous(&self.dependency_tree);
                    }
                }
                (KeyCode::PageDown, _) => {
                    self.tree_widget_state.page_down(&self.dependency_tree);
                }
                (KeyCode::PageUp, _) => {
                    self.tree_widget_state.page_up(&self.dependency_tree);
                }
                (KeyCode::Right, _) => {
                    self.tree_widget_state.expand(&self.dependency_tree);
                }
                (KeyCode::Left, _) => {
                    self.tree_widget_state.collapse(&self.dependency_tree);
                }
                _ => {}
            }
        }
    }
}

impl TuiState {
    /// Checks if needle is a subsequence of haystack (character order matching).
    fn is_subsequence_match(needle: &str, haystack: &str) -> bool {
        let needle_lower = needle.to_lowercase();
        let haystack_lower = haystack.to_lowercase();
        
        let mut needle_chars = needle_lower.chars();
        let mut current_needle_char = needle_chars.next();
        
        for haystack_char in haystack_lower.chars() {
            if let Some(n) = current_needle_char {
                if n == haystack_char {
                    current_needle_char = needle_chars.next();
                }
            }
        }
        
        current_needle_char.is_none()
    }

    /// Performs a search across all nodes in the dependency tree.
    fn perform_search(&mut self) {
        if self.search_query.is_empty() {
            self.search_results.clear();
            self.search_result_index = None;
            return;
        }

        let query = &self.search_query;
        let mut results = Vec::new();

        // Search through all nodes in the tree
        for (index, node) in self.dependency_tree.nodes.iter().enumerate() {
            let node_id = crate::core::NodeId(index);
            
            // Check if the name matches the query (using both prefix and subsequence matching)
            if Self::is_subsequence_match(query, &node.name) {
                results.push(node_id);
            }
        }

        self.search_results = results;
        self.search_result_index = if self.search_results.is_empty() { 
            None 
        } else { 
            Some(0) 
        };

        // Select the first match if available
        if let Some(&first_match) = self.search_results.first() {
            self.tree_widget_state.selected = Some(first_match);
        }
    }

    /// Moves to the next search result.
    fn next_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }

        if let Some(current_index) = self.search_result_index {
            let next_index = (current_index + 1) % self.search_results.len();
            self.search_result_index = Some(next_index);
            
            if let Some(&node_id) = self.search_results.get(next_index) {
                self.tree_widget_state.selected = Some(node_id);
            }
        }
    }

    /// Moves to the previous search result.
    fn prev_search_result(&mut self) {
        if self.search_results.is_empty() {
            return;
        }

        if let Some(current_index) = self.search_result_index {
            let prev_index = if current_index == 0 {
                self.search_results.len() - 1
            } else {
                current_index - 1
            };
            self.search_result_index = Some(prev_index);
            
            if let Some(&node_id) = self.search_results.get(prev_index) {
                self.tree_widget_state.selected = Some(node_id);
            }
        }
    }

    /// Clears the search state.
    fn clear_search(&mut self) {
        self.search_active = false;
        self.search_query.clear();
        self.search_results.clear();
        self.search_result_index = None;
    }
}
