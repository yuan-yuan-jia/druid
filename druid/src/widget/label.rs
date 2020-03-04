// Copyright 2019 The xi-editor Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! A label widget.

use crate::kurbo::{Point, Rect, Size};
use crate::piet::{
    Color, FontBuilder, PietText, PietTextLayout, RenderContext, Text, TextLayout,
    TextLayoutBuilder, UnitPoint,
};
use crate::theme;
use crate::{
    BoxConstraints, Data, Env, Event, EventCtx, KeyOrValue, LayoutCtx, LifeCycle, LifeCycleCtx,
    LocalizedString, PaintCtx, UpdateCtx, Widget,
};

/// The text for the label
pub enum LabelText<T> {
    /// Localized string that will be resolved through `Env`.
    Localized(LocalizedString<T>),
    /// Specific text
    Specific(String),
    /// The provided closure is called on update, and its return
    /// value is used as the text for the label.
    Dynamic(Dynamic<T>),
}

/// Text that is computed dynamically.
#[doc(hidden)]
pub struct Dynamic<T> {
    f: Box<dyn Fn(&T, &Env) -> String>,
    resolved: String,
}

/// A label that displays some text.
pub struct Label<T> {
    text: LabelText<T>,
    align: UnitPoint,
    color: KeyOrValue<Color>,
    size: KeyOrValue<f64>,
}

impl<T: Data> Label<T> {
    /// Construct a new Label widget.
    ///
    /// ```
    /// use druid::LocalizedString;
    /// use druid::widget::Label;
    ///
    /// // Construct a new Label using static string.
    /// let _: Label<u32> = Label::new("Hello world");
    ///
    /// // Construct a new Label using localized string.
    /// let text = LocalizedString::new("hello-counter").with_arg("count", |data: &u32, _env| (*data).into());
    /// let _: Label<u32> = Label::new(text);
    ///
    /// // Construct a new dynamic Label. Text will be updated when data changes.
    /// let _: Label<u32> = Label::new(|data: &u32, _env: &_| format!("Hello world: {}", data));
    /// ```
    pub fn new(text: impl Into<LabelText<T>>) -> Self {
        let text = text.into();
        Self {
            text,
            align: UnitPoint::LEFT,
            color: theme::LABEL_COLOR.into(),
            size: theme::TEXT_SIZE_NORMAL.into(),
        }
    }

    /// Set text alignment.
    pub fn text_align(mut self, align: UnitPoint) -> Self {
        self.align = align;
        self
    }

    /// Set text color.
    pub fn text_color(mut self, color: impl Into<KeyOrValue<Color>>) -> Self {
        self.color = color.into();
        self
    }

    /// Set text size.
    pub fn text_size(mut self, size: impl Into<KeyOrValue<f64>>) -> Self {
        self.size = size.into();
        self
    }

    fn get_layout(&mut self, t: &mut PietText, env: &Env) -> PietTextLayout {
        let font_name = env.get(theme::FONT_NAME);
        let font_size = self.size.resolve(env);

        // TODO: caching of both the format and the layout
        let font = t.new_font_by_name(font_name, font_size).build().unwrap();
        self.text
            .with_display_text(|text| t.new_text_layout(&font, &text).build().unwrap())
    }
}

impl<T> Dynamic<T> {
    fn resolve(&mut self, data: &T, env: &Env) -> bool {
        let new = (self.f)(data, env);
        let changed = new != self.resolved;
        self.resolved = new;
        changed
    }
}

impl<T: Data> LabelText<T> {
    /// Call callback with the text that should be displayed.
    pub fn with_display_text<V>(&self, mut cb: impl FnMut(&str) -> V) -> V {
        match self {
            LabelText::Specific(s) => cb(s.as_str()),
            LabelText::Localized(s) => cb(s.localized_str()),
            LabelText::Dynamic(s) => cb(s.resolved.as_str()),
        }
    }

    /// Update the localization, if necessary.
    /// This ensures that localized strings are up to date.
    ///
    /// Returns `true` if the string has changed.
    pub fn resolve(&mut self, data: &T, env: &Env) -> bool {
        match self {
            LabelText::Specific(_) => false,
            LabelText::Localized(s) => s.resolve(data, env),
            LabelText::Dynamic(s) => s.resolve(data, env),
        }
    }
}

impl<T: Data> Widget<T> for Label<T> {
    fn event(&mut self, _ctx: &mut EventCtx, _event: &Event, _data: &mut T, _env: &Env) {}

    fn lifecycle(&mut self, _ctx: &mut LifeCycleCtx, event: &LifeCycle, data: &T, env: &Env) {
        if let LifeCycle::WidgetAdded = event {
            self.text.resolve(data, env);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx, old_data: &T, data: &T, env: &Env) {
        if !old_data.same(data) && self.text.resolve(data, env) {
            ctx.request_layout();
            ctx.request_paint();
        }
    }

    fn layout(
        &mut self,
        layout_ctx: &mut LayoutCtx,
        bc: &BoxConstraints,
        _data: &T,
        env: &Env,
    ) -> Size {
        bc.debug_check("Label");

        let font_size = self.size.resolve(env);
        let text_layout = self.get_layout(layout_ctx.text(), env);
        // This magical 1.2 constant helps center the text vertically in the rect it's given
        bc.constrain(Size::new(text_layout.width(), font_size * 1.2))
    }

    fn paint(&mut self, paint_ctx: &mut PaintCtx, _data: &T, env: &Env) {
        let font_size = self.size.resolve(env);
        let text_layout = self.get_layout(paint_ctx.text(), env);

        // Find the origin for the text
        let mut origin = self.align.resolve(Rect::from_origin_size(
            Point::ORIGIN,
            Size::new(
                (paint_ctx.size().width - text_layout.width()).max(0.0),
                paint_ctx.size().height + (font_size * 1.2) / 2.,
            ),
        ));

        //Make sure we don't draw the text too low
        origin.y = origin.y.min(paint_ctx.size().height);

        let color = self.color.resolve(env);

        paint_ctx.draw_text(&text_layout, origin, &color);
    }
}

impl<T> From<String> for LabelText<T> {
    fn from(src: String) -> LabelText<T> {
        LabelText::Specific(src)
    }
}

impl<T> From<&str> for LabelText<T> {
    fn from(src: &str) -> LabelText<T> {
        LabelText::Specific(src.to_string())
    }
}

impl<T> From<LocalizedString<T>> for LabelText<T> {
    fn from(src: LocalizedString<T>) -> LabelText<T> {
        LabelText::Localized(src)
    }
}

impl<T, F: Fn(&T, &Env) -> String + 'static> From<F> for LabelText<T> {
    fn from(src: F) -> LabelText<T> {
        let f = Box::new(src);
        LabelText::Dynamic(Dynamic {
            f,
            resolved: String::default(),
        })
    }
}
