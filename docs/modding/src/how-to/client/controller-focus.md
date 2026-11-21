# Controller focus on a mod's screen

A controller reaches a control on a mod's screen only if the screen registers it as a focus target.
`DrawContext::widget_button` and `widget_text_field` register theirs. For a control the mod paints itself,
call `DrawContext::focus_target(rect, kind)` each frame from `screen_render_post`; for a list that scrolls,
call `DrawContext::scroll_area(rect, amount, max)`. Both draw nothing. Native and wasm mods use the same
methods.

| `FocusKind` | What the controller does |
|---|---|
| `Button` | A clicks its middle. |
| `Slider { value, track, step }` | A takes hold and lets go; held, left and right click the track. `value` is 0..1, `track` is `[x, width]` of the clicks that set 0 and 1, `step` is the part of the track one press moves (0.25 for 5 values; 0 moves by pixels). |
| `Row` | A clicks it. The D-pad walks the rows and scrolls their list. |
| `Tab { selected }` | A clicks it. The bumpers step through the tabs from the selected one. |

```rust,ignore
fn screen_render_post(&self, host: &dyn ClientHost, event: &mut ScreenRenderPostEvent<'_>) {
    let list = self.list.lock().unwrap();
    let [x, y, w, h] = list.rect;
    for (i, row) in list.rows_in_view() {
        let rect = [x, y + i as f32 * 20.0 - list.scroll, w, 20.0];
        event.draw.text(&row.name, [rect[0] + 4.0, rect[1] + 6.0], 0xFFFF_FFFF, false);
        event.draw.focus_target(rect, FocusKind::Row);
    }
    event.draw.scroll_area([x, y, w, h], list.scroll, list.max_scroll());
}
```

- **Units.** Rectangles, `track`, `amount` and `max` are GUI pixels, as `fill` takes them. `amount` is how far
  the list is scrolled from its top, kept to `0..=max`.
- **Only what shows.** Register the rows in view, not those scrolled away or clipped. When the D-pad steps
  past the rows shown, the list gets a mouse wheel scroll (`screen_mouse_scrolled_pre`), and the focus
  goes on to the row the next frame draws.
- **Input.** A press arrives as a mouse click at the target, through the screen's mouse hooks, so a screen
  that handles the mouse needs nothing more. A slider with a `step` is clicked where its value one step on
  is: round the value a click sets to the nearest step.
- **Merging.** A target registered after a `widget_button` with exactly its rectangle replaces that button's
  target, and its kind wins: draw a tab with `widget_button`, then register it as a `Tab`.
- **Refusals.** A rectangle that is not finite or has no area registers nothing, and so does a slider whose
  value, track or step is not finite, whose track has no width, or whose step is negative.
