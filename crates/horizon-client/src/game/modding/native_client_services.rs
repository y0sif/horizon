// game-module-visibility: pub
//! Client draw operations through the existing GUI drawing context.

use super::Game;
use horizon_client::widgets::{self, Kind};
use horizon_client::{
    hud::{HudBatch, rgba},
    hud_layout::GuiDraw,
    text::Text,
};
use horizon_items::items::ItemStack as EngineStack;
use mod_api::FocusKind;
use mod_api::{TextFieldLayout, TextFieldView};

impl Game<'_> {
    /// Paint GUI extraction calls through this window's atlas, including player faces.
    pub(super) fn native_gui_paint(&self, batch: &mut HudBatch, draws: &[GuiDraw]) {
        horizon_client::hud_layout::paint(batch, &self.gui.atlas, draws);
    }
    /// `GuiGraphics.fillGradient` for [`mod_api::DrawContext::fill_gradient`]: `rect` is
    /// `[x, y, w, h]` and `top` / `bottom` are ARGB at `y` / `y + h`. Zero area,
    /// one-axis inversion or non-finite input draws nothing; both axes inverted flips the colours.
    pub(super) fn native_fill_gradient(
        &self,
        batch: &mut HudBatch,
        rect: [f32; 4],
        top: u32,
        bottom: u32,
    ) {
        let [x, y, w, h] = rect;
        if !rect.iter().all(|v| v.is_finite()) || w == 0.0 || h == 0.0 || (w > 0.0) != (h > 0.0) {
            return;
        }
        let ((x, y, w, h), top, bottom) = if w < 0.0 {
            ((x + w, y + h, -w, -h), bottom, top)
        } else {
            ((x, y, w, h), top, bottom)
        };
        let abgr = horizon_client::hud_layout::abgr;
        batch.fill_gradient(&self.gui.atlas, (x, y, w, h), abgr(top), abgr(bottom));
    }
    /// Vanilla's nine-slice widget rendering. `color` is ARGB, as `fill` uses.
    pub(super) fn native_gui_nine_slice(
        &self,
        batch: &mut HudBatch,
        sprite: &str,
        rect: [f32; 4],
        color: u32,
    ) {
        let [a, r, g, b] = color.to_be_bytes();
        horizon_client::ui::nine_slice(
            batch,
            &self.gui.atlas,
            sprite,
            rect[0],
            rect[1],
            rect[2],
            rect[3],
            rgba(r, g, b, a),
        );
    }
    /// A mod's button, drawn as the game's screens draw theirs (`ui::button`).
    pub(super) fn native_widget_button(
        &self,
        batch: &mut HudBatch,
        [x, y, w, h]: [f32; 4],
        label: &Text,
        active: bool,
        highlighted: bool,
    ) {
        horizon_client::ui::button(
            batch,
            &self.gui.atlas,
            x,
            y,
            w,
            h,
            label,
            active,
            highlighted,
        );
    }
    /// A mod's text field, drawn as the game's screens draw theirs (`ui::EditBox`), from the
    /// state the mod keeps; answers the scroll it drew with and its character boundaries.
    pub(super) fn native_widget_text_field(
        &self,
        batch: &mut HudBatch,
        field: &TextFieldView,
    ) -> TextFieldLayout {
        let [x, y, w, h] = field.rect;
        let mut edit = horizon_client::ui::EditBox::new(x, y, w, h);
        edit.max_length = field.max_length;
        edit.value = field.value.clone();
        let len = horizon_client::ui::char_len(&edit.value);
        edit.cursor = field.cursor.min(len);
        edit.highlight = field.highlight.min(len);
        edit.set_scroll(field.scroll);
        edit.bordered = field.bordered;
        edit.focused = field.focused;
        if field.focused {
            self.pad_mod_field_drawn();
        }
        let [a, r, g, b] = field.text_color.to_be_bytes();
        edit.text_color = rgba(r, g, b, a);
        edit.suggestion = field.suggestion.clone();
        edit.message = field.message.clone();
        edit.draw_with_caret(batch, &self.gui.atlas, None, field.caret_shown);
        let (scroll, edges) = edit.shown_edges(&self.gui.atlas);
        TextFieldLayout::new(scroll, edges)
    }
    /// A vanilla tooltip at the cursor, in GUI pixels.
    pub(super) fn native_gui_tooltip(&self, batch: &mut HudBatch, lines: &[Text], at: [f32; 2]) {
        horizon_client::ui::tooltip(batch, &self.gui.atlas, lines, (at[0], at[1]));
    }

    /// Draw an item stack with custom count/decoration text (or None) at `at` in GUI units.
    pub(super) fn native_gui_draw_item_with(
        &self,
        batch: &mut HudBatch,
        stack: &EngineStack,
        at: [f32; 2],
        text: Option<&str>,
    ) {
        self.draw_item_model(batch, stack, at[0], at[1]);
        self.draw_item_decorations(batch, stack, at[0], at[1], text.map(str::to_owned));
    }
    /// Whether a specific key is currently held down.
    /// `key` is the vanilla physical key name (`key.keyboard.left.shift`, for example).
    /// Unknown names are not down. Modifier combinations use `ClientHost::modifiers`.
    pub(super) fn is_key_down(&self, key: &str) -> bool {
        horizon_client::keybinds::key_code(key).is_some_and(|key| self.physical_keys.contains(&key))
    }
    /// A control a mod paints itself, for a controller's focus this frame
    /// ([`mod_api::DrawContext::focus_target`]).
    pub(super) fn native_focus_target(&self, rect: [f32; 4], kind: FocusKind) {
        focus_target(rect, kind);
    }
    /// A mod's list that scrolls, for a controller's focus this frame
    /// ([`mod_api::DrawContext::scroll_area`]).
    pub(super) fn native_scroll_area(&self, rect: [f32; 4], amount: f32, max: f32) {
        scroll_area(rect, amount, max);
    }
}

/// A mod's focus target as the engine's, registered unless the API refuses it (a rectangle
/// `widgets::target_as` drops, a slider that can't be stepped or whose track leaves `rect`); a
/// slider's value and step are kept to 0..1.
fn focus_target(rect: [f32; 4], kind: FocusKind) {
    let kind = match kind {
        FocusKind::Button => Kind::Button,
        FocusKind::Row => Kind::Row,
        FocusKind::Tab { selected } => Kind::Tab(selected),
        FocusKind::Slider { value, track, step } => {
            let finite = [value, track[0], track[1], step]
                .iter()
                .all(|v| v.is_finite());
            let inside =
                track[0] >= rect[0] - 0.5 && track[0] + track[1] <= rect[0] + rect[2] + 0.5;
            if !finite || !inside || track[1] <= 0.0 || step < 0.0 {
                return;
            }
            Kind::Slider {
                value: value.clamp(0.0, 1.0),
                track,
                step: step.min(1.0),
            }
        }
        _ => return,
    };
    widgets::target_as(rect, kind);
}

/// A mod's scrolling list, registered when its input is finite, its rectangle has an area and
/// `max` isn't negative, scrolled no further than `0..=max`.
fn scroll_area(rect: [f32; 4], amount: f32, max: f32) {
    let finite = rect.iter().chain([&amount, &max]).all(|v| v.is_finite());
    if !finite || rect[2] <= 0.0 || rect[3] <= 0.0 || max < 0.0 {
        return;
    }
    widgets::scroll_area(rect, amount.clamp(0.0, max), max);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "daily"]
    fn tooltip_helpers_read_live_keys_and_draw_count_overrides() {
        super::super::native_entity_refs::tests::with_client_game(|game| {
            use winit::keyboard::KeyCode;

            game.physical_keys.insert(KeyCode::ShiftLeft);
            assert!(game.is_key_down("key.keyboard.left.shift"));
            assert!(!game.is_key_down("key.keyboard.right.shift"));
            assert!(!game.is_key_down("not-a-key"));
            game.physical_keys.clear();
            assert!(!game.is_key_down("key.keyboard.left.shift"));

            let stack = EngineStack::new("minecraft:diamond", 5);
            let mut automatic = HudBatch::scratch([320, 240]);
            game.native_gui_draw_item_with(&mut automatic, &stack, [16.0, 32.0], None);
            let mut custom = HudBatch::scratch([320, 240]);
            game.native_gui_draw_item_with(&mut custom, &stack, [16.0, 32.0], Some("7"));
            assert!(!automatic.vertices.is_empty());
            assert_ne!(
                bytemuck::cast_slice::<_, u8>(&automatic.vertices),
                bytemuck::cast_slice::<_, u8>(&custom.vertices),
                "custom count text must reach the actual GUI geometry"
            );
        });
    }

    /// A mod's widgets draw with the game's own: a button as `ui::button` draws it, and a text
    /// field as `ui::EditBox` draws it, answering where each shown character ends so a press
    /// lands on it, and scrolling on to keep the cursor in view.
    #[test]
    #[ignore = "daily"]
    fn mod_widgets_draw_as_the_games_own() {
        use mod_api::client::widgets::{TextField, Widgets};
        super::super::native_entity_refs::tests::with_client_game(|game| {
            let label = Text::literal("Done");
            let mut ours = HudBatch::scratch([320, 240]);
            game.native_widget_button(&mut ours, [10.0, 10.0, 100.0, 20.0], &label, true, true);
            let mut games = HudBatch::scratch([320, 240]);
            horizon_client::ui::button(
                &mut games,
                &game.gui.atlas,
                10.0,
                10.0,
                100.0,
                20.0,
                &label,
                true,
                true,
            );
            assert!(!ours.vertices.is_empty());
            assert_eq!(
                bytemuck::cast_slice::<_, u8>(&ours.vertices),
                bytemuck::cast_slice::<_, u8>(&games.vertices)
            );

            // The view a mod's group sends for a focused field, as `Widgets::draw` builds it.
            struct Draws<'a, 'g>(&'a super::Game<'g>, HudBatch, Vec<TextFieldView>);
            impl mod_api::DrawContext for Draws<'_, '_> {
                fn widget_text_field(&mut self, field: &TextFieldView) -> TextFieldLayout {
                    self.2.push(field.clone());
                    self.0.native_widget_text_field(&mut self.1, field)
                }
            }
            let mut widgets = Widgets::new();
            let id = widgets.add_text_field(TextField::new([10.0, 40.0, 60.0, 20.0], 64));
            let long = "a long name that does not fit in sixty pixels";
            widgets.text_field_mut(id).unwrap().set_value(long);
            widgets.set_focus(&*game, Some(id));
            let mut draws = Draws(game, HudBatch::scratch([320, 240]), Vec::new());
            widgets.draw(&mut draws, &*game, [0.0, 0.0]).unwrap();
            let view = draws.2[0].clone();
            assert!(!draws.1.vertices.is_empty());
            let layout = game.native_widget_text_field(&mut HudBatch::scratch([320, 240]), &view);
            // The cursor is at the end, so the field scrolled on to show it.
            assert!(layout.scroll > 0);
            let shown = layout.edges.len() - 1;
            assert_eq!(layout.scroll + shown, long.chars().count());
            assert_eq!(
                layout.edges[0], 14.0,
                "4 pixels in from a bordered field's left"
            );
            assert!(layout.edges.windows(2).all(|w| w[0] < w[1]));
            assert!(*layout.edges.last().unwrap() <= 10.0 + 60.0 - 4.0);
        });
    }

    /// What a mod registers is what a controller's focus finds on the next frame, with the
    /// engine's kind; a target over a button of the same rectangle takes its place.
    #[test]
    fn a_mods_focus_targets_and_lists_reach_the_focus() {
        use mod_api::FocusKind as F;
        let _ = widgets::take();
        let at = |y| [0.0, y, 150.0, 20.0];
        let slider = |value, step| F::Slider {
            value,
            track: [4.0, 142.0],
            step,
        };
        let bad = |value, width, step| F::Slider {
            value,
            track: [4.0, width],
            step,
        };
        let engine = |value, step| Kind::Slider {
            value,
            track: [4.0, 142.0],
            step,
        };
        let kept = [
            ("a slider", at(0.0), slider(0.5, 0.25), engine(0.5, 0.25)),
            ("clamped", at(30.0), slider(-2.0, 3.0), engine(0.0, 1.0)),
            ("a row", at(60.0), F::Row, Kind::Row),
            ("tab", at(90.0), F::Tab { selected: true }, Kind::Tab(true)),
            ("a button", at(120.0), F::Button, Kind::Button),
            ("on a labelled button", at(150.0), F::Row, Kind::Row),
        ];
        let dropped = [
            ("a NaN rectangle", [f32::NAN, 180.0, 150.0, 20.0], F::Button),
            ("no track", at(210.0), bad(0.5, 0.0, 0.0)),
            ("stepping back", at(240.0), bad(0.5, 142.0, -0.1)),
            ("a NaN value", at(270.0), bad(f32::NAN, 142.0, 0.0)),
            (
                "a track relative to its rect",
                [100.0, 300.0, 150.0, 20.0],
                slider(0.5, 0.0),
            ),
        ];
        let list = |y| [0.0, y, 200.0, 100.0];
        // (amount, max) to what the list can still scroll: (amount, up, down).
        let lists = [
            ("part way", list(0.0), 40.0, 100.0, (40.0, true, true)),
            ("at the top", list(100.0), 0.0, 100.0, (0.0, false, true)),
            ("past max", list(200.0), 150.0, 100.0, (100.0, true, false)),
        ];
        let refused = [
            ("a negative max", list(300.0), 0.0, -5.0),
            ("a NaN amount", list(400.0), f32::NAN, 100.0),
            ("no height", [0.0, 500.0, 200.0, 0.0], 0.0, 100.0),
        ];
        widgets::record("Next", at(150.0), true);
        for (_, rect, kind, _) in kept {
            focus_target(rect, kind);
        }
        for (_, rect, kind) in dropped {
            focus_target(rect, kind);
        }
        for (_, rect, amount, max, _) in lists {
            scroll_area(rect, amount, max);
        }
        for (_, rect, amount, max) in refused {
            scroll_area(rect, amount, max);
        }
        widgets::end_frame(1);

        let found = super::super::key_focus::drawn_targets();
        assert_eq!(found.len(), kept.len(), "the labelled button merged");
        for (row, rect, _) in dropped {
            assert!(found.iter().all(|t| t.rect[1] != rect[1]), "{row}");
        }
        for (row, rect, _, want) in kept {
            let got = found.iter().find(|t| t.rect == rect).map(|t| t.kind);
            assert_eq!(got, Some(want), "{row}");
        }
        let areas = widgets::last_scroll_areas();
        for (row, rect, _, _) in refused {
            assert!(areas.iter().all(|a| a.rect != rect), "{row}");
        }
        for (row, rect, _, _, want) in lists {
            let got = areas.iter().find(|a| a.rect == rect);
            assert_eq!(got.map(|a| (a.amount, a.up, a.down)), Some(want), "{row}");
        }
    }
}
