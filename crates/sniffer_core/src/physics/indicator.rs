use crate::style::{Dimension, FlexDirection};
use crate::types::Element;

#[allow(clippy::cast_possible_wrap)]
pub fn update_indicator_dots_in_element(
    element: &mut Element,
    active_page: i32,
    vmin_px: f32,
) -> bool {
    if let Element::Container {
        id,
        children,
        style,
    } = element
    {
        if let Some(id_str) = id {
            if id_str == "page_indicator_container" {
                let is_landscape = style.flex_direction == FlexDirection::Column;
                let mut changed = false;
                for (p, child_el) in children.iter_mut().enumerate() {
                    let is_active = (p as i32) == active_page;
                    if let Element::Container {
                        style: dot_style, ..
                    } = child_el
                    {
                        let new_color = Some(if is_active { 0xFF00_E5FF } else { 0x44FF_FFFF });
                        if dot_style.background_color != new_color {
                            dot_style.background_color = new_color;
                            changed = true;
                        }
                        if is_landscape {
                            let new_h = Dimension::Pixels(if is_active {
                                3.6 * vmin_px
                            } else {
                                1.6 * vmin_px
                            });
                            let new_w = Dimension::Pixels(1.6 * vmin_px);
                            if dot_style.height != new_h || dot_style.width != new_w {
                                dot_style.height = new_h;
                                dot_style.width = new_w;
                                changed = true;
                            }
                        } else {
                            let new_w = Dimension::Pixels(if is_active {
                                3.6 * vmin_px
                            } else {
                                1.6 * vmin_px
                            });
                            let new_h = Dimension::Pixels(1.6 * vmin_px);
                            if dot_style.width != new_w || dot_style.height != new_h {
                                dot_style.width = new_w;
                                dot_style.height = new_h;
                                changed = true;
                            }
                        }
                    }
                }
                return changed;
            }
        }
        for child in children {
            if update_indicator_dots_in_element(child, active_page, vmin_px) {
                return true;
            }
        }
    } else if let Element::ScrollView { children, .. } = element {
        for child in children {
            if update_indicator_dots_in_element(child, active_page, vmin_px) {
                return true;
            }
        }
    }
    false
}
