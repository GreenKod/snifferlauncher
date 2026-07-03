use crate::core::text_measure::{DEFAULT_FONT, TextMeasurer};
use crate::core::types::Element;
use crate::core::{Point, Rect, Size};
use taffy::prelude::*;

pub type LayoutChildren = Vec<LayoutNode>;

#[derive(Clone, Debug)]
pub struct LayoutNode {
    pub element: Element,
    pub rect: Rect,
    pub children: LayoutChildren,
}

impl LayoutNode {
    #[must_use]
    pub const fn new(element: Element, rect: Rect) -> Self {
        Self {
            element,
            rect,
            children: Vec::new(),
        }
    }

    #[must_use]
    pub fn hit_test(&self, point: Point) -> Option<usize> {
        for (i, child) in self.children.iter().enumerate() {
            if child.rect.contains(point) {
                return Some(i);
            }
        }
        None
    }
}

fn build_taffy_tree(taffy: &mut TaffyTree, element: &Element, measurer: &TextMeasurer) -> NodeId {
    let mut style: Style = Style::default();
    let el_style = element.style();

    style.display = el_style.display.into();
    style.position = el_style.position.into();
    style.inset = taffy::geometry::Rect {
        left: el_style.left.into(),
        right: el_style.right.into(),
        top: el_style.top.into(),
        bottom: el_style.bottom.into(),
    };

    style.flex_direction = el_style.flex_direction.into();
    style.flex_wrap = el_style.flex_wrap.into();
    style.justify_content = Some(el_style.justify_content.into());
    style.align_items = Some(el_style.align_items.into());
    
    style.flex_grow = el_style.flex_grow;
    style.flex_shrink = el_style.flex_shrink;
    style.flex_basis = el_style.flex_basis.into();

    style.size = taffy::geometry::Size {
        width: el_style.width.into(),
        height: el_style.height.into(),
    };
    
    style.min_size = taffy::geometry::Size {
        width: el_style.min_width.into(),
        height: el_style.min_height.into(),
    };

    style.max_size = taffy::geometry::Size {
        width: el_style.max_width.into(),
        height: el_style.max_height.into(),
    };
    style.padding = el_style.padding.into();
    style.margin = el_style.margin.into();
    style.gap = taffy::geometry::Size {
        width: LengthPercentage::Length(el_style.gap),
        height: LengthPercentage::Length(el_style.gap),
    };
    
    // Disable shrinking so items in ScrollView retain their specified height
    // UNLESS the user explicitly sets flex_shrink (e.g. not 0.0)
    // Wait, the default is 0.0 anyway. Let's just rely on the mapped flex_shrink above.
    
    if let Element::ScrollView { .. } = element {
        // Taffy'e bu konteynerin scroll edilebilir olduğunu (içeriği sınırlandırmaması gerektiğini) belirtelim
        style.overflow = taffy::geometry::Point { x: taffy::style::Overflow::Scroll, y: taffy::style::Overflow::Scroll };
    } else if el_style.overflow_hidden {
        style.overflow = taffy::geometry::Point { x: taffy::style::Overflow::Hidden, y: taffy::style::Overflow::Hidden };
    }

    match element {
        Element::Container { children, .. } | Element::ScrollView { children, .. } => {
            let child_nodes: Vec<_> = children
                .iter()
                .map(|c| build_taffy_tree(taffy, c, measurer))
                .collect();
            taffy.new_with_children(style, &child_nodes).unwrap()
        }
        Element::Label { text, .. } => {
            let (w, h) = measurer.measure(text, el_style.text_size);
            if el_style.width == crate::core::style::Dimension::Auto {
                style.size.width =
                    Dimension::Length(w + el_style.padding.left + el_style.padding.right);
            }
            if el_style.height == crate::core::style::Dimension::Auto {
                style.size.height =
                    Dimension::Length(h + el_style.padding.top + el_style.padding.bottom);
            }
            taffy.new_leaf(style).unwrap()
        }
        Element::TextInput { value, focused, .. } => {
            let display_text = if *focused {
                format!("{value}_")
            } else {
                value.clone()
            };
            let (w, h) = measurer.measure(&display_text, el_style.text_size);
            if el_style.width == crate::core::style::Dimension::Auto {
                style.size.width =
                    Dimension::Length(w + el_style.padding.left + el_style.padding.right);
            }
            if el_style.height == crate::core::style::Dimension::Auto {
                let h_clamped = h.max(el_style.text_size * 1.5);
                style.size.height =
                    Dimension::Length(h_clamped + el_style.padding.top + el_style.padding.bottom);
            }
            taffy.new_leaf(style).unwrap()
        }
        Element::Image { .. } => {
            if el_style.width == crate::core::style::Dimension::Auto {
                style.size.width = Dimension::Length(100.0);
            }
            if el_style.height == crate::core::style::Dimension::Auto {
                style.size.height = Dimension::Length(100.0);
            }
            taffy.new_leaf(style).unwrap()
        }
        Element::Checkbox { .. } => {
            if el_style.width == crate::core::style::Dimension::Auto {
                style.size.width = Dimension::Length(24.0);
            }
            if el_style.height == crate::core::style::Dimension::Auto {
                style.size.height = Dimension::Length(24.0);
            }
            taffy.new_leaf(style).unwrap()
        }
        Element::Slider { .. } | Element::ProgressBar { .. } => {
            if el_style.width == crate::core::style::Dimension::Auto {
                style.size.width = Dimension::Length(100.0);
            }
            if el_style.height == crate::core::style::Dimension::Auto {
                style.size.height = Dimension::Length(24.0);
            }
            taffy.new_leaf(style).unwrap()
        }
    }
}

fn resolve_layout(
    taffy: &TaffyTree,
    node: NodeId,
    element: &Element,
    parent_x: f32,
    parent_y: f32,
) -> LayoutNode {
    let layout = taffy.layout(node).unwrap();

    let abs_x = parent_x + layout.location.x;
    let abs_y = parent_y + layout.location.y;
    let rect = Rect::new(abs_x, abs_y, layout.size.width, layout.size.height);

    let mut children = Vec::new();
    if let Element::Container { children: el_children, .. } | Element::ScrollView { children: el_children, .. } = element {
        let child_nodes = taffy.children(node).unwrap();
        for (child_el, child_node) in el_children.iter().zip(child_nodes.iter()) {
            children.push(resolve_layout(taffy, *child_node, child_el, abs_x, abs_y));
        }
    }

    LayoutNode {
        element: element.clone(),
        rect,
        children,
    }
}

/// Calculates the layout for the entire UI tree.
///
/// # Panics
///
/// Panics if the default font cannot be loaded for measuring text.
#[must_use]
pub fn calculate_layout(
    element: &Element,
    parent_size: Size,
    x_offset: f32,
    y_offset: f32,
) -> LayoutNode {
    let mut taffy = TaffyTree::new();
    let measurer =
        TextMeasurer::new(DEFAULT_FONT).expect("Failed to load default font for measuring");

    let root = build_taffy_tree(&mut taffy, element, &measurer);

    // Create an invisible root style to enforce parent size
    let root_style = Style {
        size: taffy::geometry::Size {
            width: Dimension::Length(parent_size.width),
            height: Dimension::Length(parent_size.height),
        },
        ..Default::default()
    };
    let wrapper = taffy.new_with_children(root_style, &[root]).unwrap();

    taffy
        .compute_layout(
            wrapper,
            taffy::geometry::Size {
                width: AvailableSpace::Definite(parent_size.width),
                height: AvailableSpace::Definite(parent_size.height),
            },
        )
        .unwrap();

    // Skip the wrapper node and just resolve the root
    resolve_layout(&taffy, root, element, x_offset, y_offset)
}
