//! A controller over a native mod's own screen, which takes its input only through the screen
//! hooks: what each of the pad's inputs makes the mod hear, beside the mouse's.
use super::*;
use crate::game::input_trace::InputOrigin;
use mod_api::client::widgets::{TextField, Widgets};
use mod_api::{
    ClientHost, Flow, Hooks, ModEntry, MouseButtonPreEvent, ScreenCharTypedPreEvent,
    ScreenKeyPressedPreEvent, ScreenMouseButtonPressedPreEvent, ScreenMouseButtonReleasedPreEvent,
    ScreenMouseDraggedPreEvent, ScreenMouseScrolledPreEvent, TextFieldLayout, TextFieldView,
};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// The mod whose screen is open.
const PROBE: &str = "pad_probe";

/// Writes down the raw button hook and the mod screen's input hooks as they fire, and keeps
/// Escape when told to.
#[derive(Clone, Default)]
struct Probe {
    heard: Arc<Mutex<Vec<String>>>,
    keep_escape: Arc<AtomicBool>,
}

impl Probe {
    fn hear(&self, screen: &str, what: String) {
        if screen == "mod" {
            self.heard.lock().unwrap().push(what);
        }
    }
    fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.heard.lock().unwrap())
    }
}

#[mod_api::__subscriptions]
impl Hooks for Probe {
    fn mouse_button_pre(&self, _: &mut dyn ClientHost, event: &mut MouseButtonPreEvent) -> Flow {
        let state = if event.pressed { "down" } else { "up" };
        self.hear("mod", format!("raw {} {state}", event.button));
        Flow::Continue
    }
    fn screen_mouse_button_pressed_pre(
        &self,
        _: &dyn ClientHost,
        event: &mut ScreenMouseButtonPressedPreEvent<'_>,
    ) -> Flow {
        self.hear(event.screen.id, format!("press {}", event.button));
        Flow::Continue
    }
    fn screen_mouse_button_released_pre(
        &self,
        _: &dyn ClientHost,
        event: &mut ScreenMouseButtonReleasedPreEvent<'_>,
    ) -> Flow {
        self.hear(event.screen.id, format!("release {}", event.button));
        Flow::Continue
    }
    fn screen_mouse_dragged_pre(
        &self,
        _: &dyn ClientHost,
        event: &mut ScreenMouseDraggedPreEvent<'_>,
    ) -> Flow {
        self.hear(event.screen.id, format!("drag {}", event.button));
        Flow::Continue
    }
    fn screen_mouse_scrolled_pre(
        &self,
        _: &dyn ClientHost,
        event: &mut ScreenMouseScrolledPreEvent<'_>,
    ) -> Flow {
        self.hear(event.screen.id, "scroll".to_owned());
        Flow::Continue
    }
    fn screen_key_pressed_pre(
        &self,
        _: &dyn ClientHost,
        event: &mut ScreenKeyPressedPreEvent<'_>,
    ) -> Flow {
        self.hear(event.screen.id, format!("key {}", event.key));
        if event.key == "key.keyboard.escape" && self.keep_escape.load(Ordering::Relaxed) {
            Flow::Cancel
        } else {
            Flow::Continue
        }
    }
    fn screen_char_typed_pre(
        &self,
        _: &dyn ClientHost,
        event: &mut ScreenCharTypedPreEvent<'_>,
    ) -> Flow {
        self.hear(event.screen.id, format!("char {}", event.c));
        Flow::Continue
    }
}

/// A step of an event loop that is never asked to stop.
struct NoEventLoop;
impl EventLoopCtl for NoEventLoop {
    fn exit(&self) {}
    fn set_control_flow(&self, _: winit::event_loop::ControlFlow) {}
}

/// A mod's screen drawing its widgets with the game's own text field.
struct Draws<'a, 'g>(&'a Game<'g>, HudBatch);
impl mod_api::DrawContext for Draws<'_, '_> {
    fn widget_text_field(&mut self, field: &TextFieldView) -> TextFieldLayout {
        self.0.native_widget_text_field(&mut self.1, field)
    }
}

/// One input and what the mod hears of it, in order.
type Row = (
    &'static str,
    fn(&mut Game<'_>, &Probe, &str),
    &'static [&'static str],
);

/// The pad's clicks, drags and wheel reach the mouse hooks as the mouse's do, the keyboard
/// focus's clicks don't, the on-screen keyboard types into the screen's focused field, B asks
/// Escape first, and a press that closed the screen still gets its release.
#[test]
#[ignore = "daily"]
fn a_controller_reaches_a_mod_screens_hooks_as_the_mouse_and_keyboard_do() {
    // Winit permits only one event loop per process, even after it is dropped.
    let name = "game::pad_menu::mod_screen_tests::a_controller_reaches_a_mod_screens_hooks_as_the_mouse_and_keyboard_do";
    if crate::game::native_entity_refs::tests::run_fresh_cases(
        name,
        "HORIZON_PAD_MOD_SCREEN_CHILD",
        &["window"],
    ) {
        return;
    }
    let mut builder = winit::event_loop::EventLoop::builder();
    #[cfg(target_os = "linux")]
    {
        use winit::platform::x11::EventLoopBuilderExtX11;
        builder.with_x11().with_any_thread(true);
    }
    #[cfg(target_os = "windows")]
    {
        use winit::platform::windows::EventLoopBuilderExtWindows;
        builder.with_any_thread(true);
    }
    let event_loop = builder.build().unwrap();
    #[allow(deprecated)]
    let window = Arc::new(
        event_loop
            .create_window(
                winit::window::Window::default_attributes()
                    .with_visible(false)
                    .with_inner_size(winit::dpi::PhysicalSize::new(320, 240)),
            )
            .unwrap(),
    );
    let rows: [Row; 12] = [
        (
            "the mouse's click",
            |game, _, _| {
                let id = game.window.as_ref().unwrap().id();
                for state in [ElementState::Pressed, ElementState::Released] {
                    let event = WindowEvent::MouseInput {
                        device_id: DeviceId::dummy(),
                        state,
                        button: MouseButton::Left,
                    };
                    game.handle_window_event(&NoEventLoop, id, event, InputOrigin::User);
                }
            },
            &["raw 0 down", "press 0", "raw 0 up", "release 0"],
        ),
        (
            "the pad's click",
            |game, _, _| game.pad_click_at(&NoEventLoop, (40.0, 40.0), false),
            &["raw 0 down", "press 0", "raw 0 up", "release 0"],
        ),
        (
            "the pad's right click",
            |game, _, _| game.pad_click_at(&NoEventLoop, (40.0, 40.0), true),
            &["raw 1 down", "press 1", "raw 1 up", "release 1"],
        ),
        (
            "the pad's held press, moved",
            |game, _, _| {
                game.pad_pointer_button(&NoEventLoop, false, true);
                game.pad_pointer_to(&NoEventLoop, (60.0, 50.0));
                game.pad_pointer_button(&NoEventLoop, false, false);
            },
            &["raw 0 down", "press 0", "drag 0", "raw 0 up", "release 0"],
        ),
        (
            "the pad's wheel",
            |game, _, _| game.pad_wheel(&NoEventLoop, 0.0, 1.0),
            &["scroll"],
        ),
        (
            "Enter on the keyboard focus's button",
            |game, _, row| {
                let rect = [20.0, 30.0, 40.0, 20.0];
                widgets::record("Probe", rect, true);
                widgets::end_frame(u64::MAX);
                game.pad.menu.keyboard = true;
                game.pad.menu.focus = Some(Target {
                    rect,
                    kind: Kind::Button,
                });
                assert!(game.key_focus_key(&NoEventLoop, KeyCode::Enter), "{row}");
            },
            &[],
        ),
        (
            "the right stick, the keyboard's focus left on",
            |game, _, row| {
                assert!(game.pad.menu.keyboard, "{row}: left on by the row before");
                game.pad_stick_wheel(&NoEventLoop, true, &[], [0.0, 1.0]);
                game.pad.menu.keyboard = false;
            },
            &["scroll"],
        ),
        (
            "the keyboard focus's step scrolling a list on, frames later",
            |game, _, row| {
                // Rows 20 high in a list 40 high, the one below the focus never all in view.
                let frame = |amount: f32, n| {
                    widgets::scroll_area([10.0, 20.0, 60.0, 40.0], amount, 30.0);
                    for y in [30.0, 50.0] {
                        widgets::record("Row", [20.0, y - amount, 40.0, 20.0], true);
                    }
                    widgets::end_frame(n);
                };
                frame(0.0, 1);
                game.pad.menu.keyboard = true;
                game.pad.menu.focus = Some(Target {
                    rect: [20.0, 30.0, 40.0, 20.0],
                    kind: Kind::Button,
                });
                assert!(
                    game.key_focus_key(&NoEventLoop, KeyCode::ArrowDown),
                    "{row}"
                );
                frame(5.0, 2);
                frame(5.0, 3);
                let targets = game.pad_targets();
                game.pad_follow_focus(&NoEventLoop, &targets);
                assert!(game.pad.menu.pending.is_some(), "{row}: scrolled again");
                game.pad.menu.keyboard = false;
            },
            &[],
        ),
        (
            "the on-screen keyboard over a focused field",
            |game, _, row| {
                assert!(!game.pad_has_typing_screen(), "{row}: no field focused yet");
                let mut fields = Widgets::new();
                let id = fields.add_text_field(TextField::new([10.0, 40.0, 60.0, 20.0], 64));
                fields.set_focus(&*game, Some(id));
                let mut draws = Draws(game, HudBatch::scratch([320, 240]));
                fields.draw(&mut draws, &*game, [0.0, 0.0]).unwrap();
                assert!(
                    game.pad_has_typing_screen(),
                    "{row}: the field is drawn focused"
                );
                game.pad.menu.osk = Some(Keyboard::new(false, false));
                game.pad_osk_output(osk::Output::Text("a".into()));
                game.pad_osk_output(osk::Output::Backspace);
                assert!(game.pad.menu.osk.is_some(), "{row}: open while typing");
                game.pad_osk_output(osk::Output::Done);
                assert!(game.pad.menu.osk.is_none(), "{row}: Done closes it");
            },
            &[
                "char a",
                "key key.keyboard.backspace",
                "key key.keyboard.enter",
            ],
        ),
        (
            "B, Escape kept by the screen",
            |game, probe, row| {
                probe.keep_escape.store(true, Ordering::Relaxed);
                game.pad_back().unwrap();
                assert_eq!(game.screen, Screen::Mod, "{row}");
            },
            &["key key.keyboard.escape"],
        ),
        (
            "B, Escape let through",
            |game, probe, row| {
                probe.keep_escape.store(false, Ordering::Relaxed);
                game.pad_back().unwrap();
                assert_eq!(game.screen, Screen::Playing, "{row}");
            },
            &["key key.keyboard.escape"],
        ),
        (
            "the pad's press that closed the screen",
            |game, _, row| {
                game.native_mod_screen = Some(PROBE.into());
                game.screen = Screen::Mod;
                game.pad_pointer_button(&NoEventLoop, false, true);
                game.close_mod_screen();
                game.pad_cancel_menu(&NoEventLoop);
                assert_eq!(game.screen, Screen::Playing, "{row}");
            },
            &["raw 0 down", "press 0", "raw 0 up"],
        ),
    ];
    let probe = Probe::default();
    let mods: mod_api::Mods = vec![ModEntry::new(PROBE, Box::new(probe.clone()))].into();
    mod_platform::testing::with_mods(mods, || {
        crate::game::native_entity_refs::tests::with_client_game(|game| {
            game.options.offscreen = true;
            game.window = Some(window.clone());
            game.renderer = Some(game.new_renderer(window.clone()).unwrap());
            game.title = None;
            game.player = Some(
                horizon_client::motion::PlayerMotion::new(
                    game.motion,
                    horizon_core::geometry::Vec3::ZERO,
                )
                .unwrap(),
            );
            game.native_mod_screen = Some(PROBE.into());
            game.screen = Screen::Mod;
            probe.take();
            for (row, act, expected) in rows {
                act(game, &probe, row);
                assert_eq!(probe.take(), expected, "{row}");
            }
        });
    });
}
