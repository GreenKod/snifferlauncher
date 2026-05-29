use crate::core::geometry::{Point, Rect, Size};
use crate::core::style::{Dimension, FlexDirection, JustifyContent, AlignItems};
use crate::core::component::Element;

#[derive(Clone, Debug)]
pub struct LayoutNode {
    pub rect: Rect,
    pub children: Vec<LayoutNode>,
}

// Estimates text size for auto-sizing labels
fn estimate_text_size(text: &str, text_size: f32) -> Size {
    let char_width = text_size;
    let gap = text_size * 0.1;
    let width = if text.is_empty() {
        0.0
    } else {
        text.len() as f32 * char_width + (text.len() - 1) as f32 * gap
    };
    Size::new(width, text_size)
}

impl LayoutNode {
    pub fn new(rect: Rect) -> Self {
        Self { rect, children: Vec::new() }
    }

    pub fn hit_test(&self, point: Point) -> Option<usize> {
        // Simple hit-testing of child indices relative to this node
        for (i, child) in self.children.iter().enumerate() {
            if child.rect.contains(point) {
                return Some(i);
            }
        }
        None
    }
}

pub fn calculate_layout(
    element: &Element,
    parent_size: Size,
    x_offset: f32,
    y_offset: f32,
) -> LayoutNode {
    let style = element.style();
    
    // 1. Resolve self width and height
    let resolved_width = match style.width {
        Dimension::Pixels(w) => w,
        Dimension::Percent(p) => parent_size.width * (p / 100.0),
        Dimension::Auto => parent_size.width, // default to stretch
    };

    let resolved_height = match style.height {
        Dimension::Pixels(h) => h,
        Dimension::Percent(p) => parent_size.height * (p / 100.0),
        Dimension::Auto => match element {
            Element::Label { text, .. } => estimate_text_size(text, style.text_size).height,
            Element::Icon { .. } => 56.0, // Standard icon size
            Element::Button { .. } => 86.0, // Standard button height
            Element::Container { .. } => 0.0, // Will be computed after children layout
        },
    };

    let self_size = Size::new(resolved_width, resolved_height);

    match element {
        Element::Container { children, .. } => {
            let inner_width = (self_size.width - style.padding.left - style.padding.right).max(0.0);
            let inner_height = (self_size.height - style.padding.top - style.padding.bottom).max(0.0);

            // First pass: layout children to calculate their sizes
            let mut child_layouts = Vec::new();
            let mut total_content_height = 0.0;
            let mut total_content_width = 0.0;
            
            for (i, child) in children.iter().enumerate() {
                let child_style = child.style();
                // Assign a target constraint size for the child
                let target_width = match child_style.width {
                    Dimension::Pixels(w) => w,
                    Dimension::Percent(p) => inner_width * (p / 100.0),
                    Dimension::Auto => if style.flex_direction == FlexDirection::Column && style.align_items == AlignItems::Stretch {
                        inner_width
                    } else {
                        inner_width
                    },
                };

                let target_height = match child_style.height {
                    Dimension::Pixels(h) => h,
                    Dimension::Percent(p) => inner_height * (p / 100.0),
                    Dimension::Auto => match child {
                        Element::Label { text, .. } => estimate_text_size(text, child_style.text_size).height,
                        Element::Icon { .. } => 56.0,
                        Element::Button { .. } => 86.0,
                        Element::Container { .. } => 0.0, // Temporary
                    },
                };

                let child_layout = calculate_layout(child, Size::new(target_width, target_height), 0.0, 0.0);
                
                if i > 0 {
                    total_content_height += style.gap;
                    total_content_width += style.gap;
                }
                total_content_height += child_layout.rect.height;
                total_content_width += child_layout.rect.width;
                child_layouts.push((child, child_layout));
            }

            // Resolve container's auto height/width
            let final_height = match style.height {
                Dimension::Auto => total_content_height + style.padding.top + style.padding.bottom,
                _ => self_size.height,
            };

            let final_width = self_size.width;

            let self_rect = Rect::new(x_offset, y_offset, final_width, final_height);
            let mut final_child_nodes = Vec::new();

            // Second pass: position children based on alignment and justification rules
            if style.flex_direction == FlexDirection::Column {
                let inner_height = (final_height - style.padding.top - style.padding.bottom).max(0.0);
                let remaining_space = (inner_height - total_content_height).max(0.0);

                let mut current_y = match style.justify_content {
                    JustifyContent::Start => style.padding.top,
                    JustifyContent::Center => style.padding.top + remaining_space * 0.5,
                    JustifyContent::End => style.padding.top + remaining_space,
                    JustifyContent::SpaceBetween => style.padding.top,
                };

                let spacing_factor = if style.justify_content == JustifyContent::SpaceBetween && children.len() > 1 {
                    remaining_space / (children.len() - 1) as f32
                } else {
                    0.0
                };

                for (child, mut layout) in child_layouts {
                    let child_w = layout.rect.width;
                    
                    let current_x = match style.align_items {
                        AlignItems::Start => style.padding.left,
                        AlignItems::Center => style.padding.left + (inner_width - child_w) * 0.5,
                        AlignItems::End => style.padding.left + inner_width - child_w,
                        AlignItems::Stretch => {
                            layout.rect.width = inner_width;
                            style.padding.left
                        }
                    };

                    // Re-calculate child layout recursively relative to self
                    let final_child = calculate_layout(
                        child,
                        Size::new(layout.rect.width, layout.rect.height),
                        x_offset + current_x,
                        y_offset + current_y,
                    );
                    final_child_nodes.push(final_child);

                    current_y += layout.rect.height + style.gap + spacing_factor;
                }
            } else { // Row layout
                let inner_width = (final_width - style.padding.left - style.padding.right).max(0.0);
                let remaining_space = (inner_width - total_content_width).max(0.0);

                let mut current_x = match style.justify_content {
                    JustifyContent::Start => style.padding.left,
                    JustifyContent::Center => style.padding.left + remaining_space * 0.5,
                    JustifyContent::End => style.padding.left + remaining_space,
                    JustifyContent::SpaceBetween => style.padding.left,
                };

                let spacing_factor = if style.justify_content == JustifyContent::SpaceBetween && children.len() > 1 {
                    remaining_space / (children.len() - 1) as f32
                } else {
                    0.0
                };

                for (child, mut layout) in child_layouts {
                    let child_h = layout.rect.height;

                    let current_y = match style.align_items {
                        AlignItems::Start => style.padding.top,
                        AlignItems::Center => style.padding.top + (inner_height - child_h) * 0.5,
                        AlignItems::End => style.padding.top + inner_height - child_h,
                        AlignItems::Stretch => {
                            layout.rect.height = inner_height;
                            style.padding.top
                        }
                    };

                    let final_child = calculate_layout(
                        child,
                        Size::new(layout.rect.width, layout.rect.height),
                        x_offset + current_x,
                        y_offset + current_y,
                    );
                    final_child_nodes.push(final_child);

                    current_x += layout.rect.width + style.gap + spacing_factor;
                }
            }

            LayoutNode {
                rect: self_rect,
                children: final_child_nodes,
            }
        }
        _ => {
            // Leaf nodes (Button, Label, Icon)
            LayoutNode {
                rect: Rect::new(x_offset, y_offset, self_size.width, self_size.height),
                children: Vec::new(),
            }
        }
    }
}
