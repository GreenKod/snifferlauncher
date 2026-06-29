use crate::core::style::Style;
use crate::core::ui::widget::WidgetId;

/// Fluent builder for `Widget::Button`.
///
/// # Örnek
/// ```
/// use snifferlauncher::core::ui::builder::ButtonBuilder;
/// use snifferlauncher::core::ui::widget::ids;
///
/// let widget = ButtonBuilder::new(ids::BTN_SETTINGS, "Settings")
///     .style(my_style)
///     .hovered(true)
///     .build();
/// ```
pub struct ButtonBuilder {
    id: WidgetId,
    label: String,
    style: Style,
    hovered: bool,
}

impl ButtonBuilder {
    #[must_use]
    pub fn new(id: WidgetId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            style: Style::default(),
            hovered: false,
        }
    }

    #[must_use]
    pub const fn style(mut self, s: Style) -> Self {
        self.style = s;
        self
    }

    #[must_use]
    pub const fn hovered(mut self, h: bool) -> Self {
        self.hovered = h;
        self
    }

    #[must_use]
    pub fn build(self) -> crate::core::ui::widget::Widget {
        crate::core::ui::widget::Widget::Button {
            id: self.id,
            label: self.label,
            style: self.style,
            hovered: self.hovered,
        }
    }
}

/// Fluent builder for `Widget::Label`.
pub struct LabelBuilder {
    id: Option<WidgetId>,
    text: String,
    style: Style,
    visible: bool,
}

impl LabelBuilder {
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            id: None,
            text: text.into(),
            style: Style::default(),
            visible: true,
        }
    }

    #[must_use]
    pub const fn id(mut self, id: WidgetId) -> Self {
        self.id = Some(id);
        self
    }

    #[must_use]
    pub const fn style(mut self, s: Style) -> Self {
        self.style = s;
        self
    }

    #[must_use]
    pub const fn visible(mut self, v: bool) -> Self {
        self.visible = v;
        self
    }

    #[must_use]
    pub fn build(self) -> crate::core::ui::widget::Widget {
        crate::core::ui::widget::Widget::Label {
            id: self.id,
            text: self.text,
            style: self.style,
            visible: self.visible,
        }
    }
}

/// Fluent builder for `Widget::Container`.
pub struct ContainerBuilder {
    id: Option<WidgetId>,
    style: Style,
    children: Vec<crate::core::ui::widget::Widget>,
}

impl ContainerBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self {
            id: None,
            style: Style::default(),
            children: Vec::new(),
        }
    }

    #[must_use]
    pub const fn id(mut self, id: WidgetId) -> Self {
        self.id = Some(id);
        self
    }

    #[must_use]
    pub const fn style(mut self, s: Style) -> Self {
        self.style = s;
        self
    }

    #[must_use]
    pub fn child(mut self, w: crate::core::ui::widget::Widget) -> Self {
        self.children.push(w);
        self
    }

    #[must_use]
    pub fn children(mut self, ws: Vec<crate::core::ui::widget::Widget>) -> Self {
        self.children = ws;
        self
    }

    #[must_use]
    pub fn build(self) -> crate::core::ui::widget::Widget {
        crate::core::ui::widget::Widget::Container {
            id: self.id,
            style: self.style,
            children: self.children,
        }
    }
}

impl Default for ContainerBuilder {
    fn default() -> Self {
        Self::new()
    }
}
