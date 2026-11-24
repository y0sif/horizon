//! `prepare_item_craft_result` and `prepare_smithing_result`: a mod clears or replaces a result
//! the recipe made, and the player is shown, and takes, what the mod left.
use super::native_guest_turn_tests::with_guests;
use super::native_stations_tests::{menu_key, sent, stick_guest};
use crate::game::native_entity_refs::tests::with_game;
use horizon_client::{items::ItemStack, lan::PlayerSender};
use horizon_protocol::packets::{
    play::{Clientbound as C, ContainerClick, Serverbound},
    recipes::PlaceRecipe,
};
use mod_api::{Hooks, PrepareItemCraftResultEvent, PrepareSmithingResultEvent, ServerHost};
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
    mpsc,
};

/// What the listener leaves in place of the result: nothing, or one of this item.
static ANSWER: Mutex<Option<&'static str>> = Mutex::new(None);
/// The crafting listener leaves the result as it arrived.
static KEEP: AtomicBool = AtomicBool::new(false);
/// Every ask: whether it had a `pos`, how many inputs, and the result it arrived with.
static HEARD: Mutex<Vec<(bool, usize, Option<String>)>> = Mutex::new(Vec::new());

fn item_of(stack: Option<&mod_api::ItemStack>) -> Option<String> {
    stack.and_then(|stack| mod_api::__private::item_key_id(stack.item()).map(str::to_owned))
}
fn answer() -> Option<mod_api::ItemStack> {
    let item = (*ANSWER.lock().unwrap())?;
    mod_api::ItemStack::new(mod_api::__private::item_key(item)?, 1)
}
fn heard() -> Vec<(bool, usize, Option<String>)> {
    std::mem::take(&mut *HEARD.lock().unwrap())
}

struct Lock;
#[mod_api::__subscriptions]
impl Hooks for Lock {
    fn prepare_item_craft_result(
        &self,
        _: &mut dyn ServerHost,
        event: &mut PrepareItemCraftResultEvent<'_>,
    ) {
        let arrived = item_of(event.result.as_ref());
        HEARD
            .lock()
            .unwrap()
            .push((event.pos.is_some(), event.inputs.len(), arrived));
        if !KEEP.load(Ordering::Relaxed) {
            *event.result = answer();
        }
    }
    fn prepare_smithing_result(
        &self,
        _: &mut dyn ServerHost,
        event: &mut PrepareSmithingResultEvent<'_>,
    ) {
        let arrived = item_of(event.result.as_ref());
        HEARD
            .lock()
            .unwrap()
            .push((true, event.inputs.len(), arrived));
        *event.result = answer();
    }
}

fn with_lock(test: impl FnOnce()) {
    let mods = vec![mod_api::ModEntry::new("lock", Box::new(Lock))].into();
    mod_platform::testing::with_mods(mods, test);
}

#[test]
#[ignore = "daily"]
fn an_inventory_grid_result_is_cleared_or_replaced_and_taken_as_the_mod_left_it() {
    with_lock(|| {
        with_guests(|game| {
            let click = |game: &mut crate::game::Game<'_>, slot, input| {
                // The state the client was last sent: an older one is answered with the whole
                // inventory, not with the slots the click left it wrong about.
                let state = game.lan.as_ref().unwrap().hub.guests[0].menu_state;
                game.lan_guest_packet(
                    12,
                    Serverbound::ContainerClick(ContainerClick {
                        container: 0,
                        state,
                        slot,
                        button: 0,
                        input,
                        changed: Vec::new(),
                        carried: None,
                    }),
                );
            };
            let logs = |game: &mut crate::game::Game<'_>, count| {
                let host = game.lan.as_mut().unwrap();
                host.hub.guests.update(0, |guest| {
                    guest.craft_grid[0] = Some(ItemStack::new("minecraft:oak_log", count));
                });
            };
            let state = |game: &crate::game::Game<'_>| {
                let guest = &game.lan.as_ref().unwrap().hub.guests[0];
                let name = |stack: Option<&ItemStack>| stack.map(|s| s.item.to_string());
                (
                    guest.craft_grid[0].as_ref().map_or(0, |s| s.count),
                    name(guest.craft_result.as_ref()),
                    name(guest.carried.as_ref()),
                )
            };
            let planks = Some("minecraft:oak_planks".to_owned());
            let diamond = Some("minecraft:diamond".to_owned());
            // The guest knows the planks recipe and holds logs for it, and the test reads
            // what their client is sent.
            let (tx, rx) = mpsc::channel();
            let index = &game.stations().recipe_index;
            let recipe = index.first_display("minecraft:oak_planks").unwrap();
            let mut book = game.lan.as_ref().unwrap().hub.guests[0].recipe_book.clone();
            book.add_recipes(index, &["minecraft:oak_planks".to_owned()]);
            game.lan.as_mut().unwrap().hub.guests.update(0, |guest| {
                guest.sender = PlayerSender::new(tx, Arc::default());
                guest.recipe_book = book;
                guest.inventory[0] = Some(ItemStack::new("minecraft:oak_log", 3));
            });

            // Cleared: a recipe placed from the book leaves the result empty.
            KEEP.store(false, Ordering::Relaxed);
            *ANSWER.lock().unwrap() = None;
            heard();
            game.lan_guest_packet(
                12,
                Serverbound::PlaceRecipe(PlaceRecipe {
                    container: 0,
                    recipe,
                    use_max: false,
                }),
            );
            assert_eq!(state(game), (1, None, None));
            assert_eq!(heard(), [(false, 4, planks.clone())]);

            // Cleared: the recipe's planks are heard once, with the grid, and a pickup and a
            // shift-click on the result slot take nothing and use nothing up.
            logs(game, 2);
            click(game, 0, 0);
            assert_eq!(state(game), (2, None, None));
            assert_eq!(heard(), [(false, 4, planks.clone())]);
            click(game, 0, 1);
            assert_eq!(state(game), (2, None, None));
            let held = &game.lan.as_ref().unwrap().hub.guests[0].inventory;
            let no_planks = |s: &ItemStack| &*s.item != "minecraft:oak_planks";
            assert!(held.iter().flatten().all(no_planks));
            // The grid stood, so the result was not made again and nothing was asked.
            assert_eq!(heard(), []);

            // Replaced: the take gets the mod's diamond and uses up the grid as the recipe
            // does; the result made from what is left is asked about, and replaced, again.
            *ANSWER.lock().unwrap() = Some("minecraft:diamond");
            logs(game, 3);
            click(game, 0, 0);
            assert_eq!(state(game), (2, diamond.clone(), diamond));
            assert_eq!(
                heard(),
                [(false, 4, planks.clone()), (false, 4, planks.clone())]
            );

            // Sent: one more log makes the same planks again, so the click leaves the result
            // slot as it was; the listener, which had left the planks, now clears them, and
            // the client is sent the empty slot. The first click puts the diamond down.
            KEEP.store(true, Ordering::Relaxed);
            logs(game, 1);
            click(game, 9, 0);
            assert_eq!(state(game), (1, planks.clone(), None));
            KEEP.store(false, Ordering::Relaxed);
            *ANSWER.lock().unwrap() = None;
            game.lan.as_mut().unwrap().hub.guests.update(0, |guest| {
                guest.carried = Some(ItemStack::new("minecraft:oak_log", 1));
            });
            heard();
            sent(&rx);
            click(game, 1, 0);
            assert_eq!(state(game), (2, None, None));
            assert_eq!(heard(), [(false, 4, planks)]);
            let emptied = |packet: &C| {
                matches!(packet, C::ContainerSetSlot(s)
                    if s.container == 0 && s.slot == 0 && s.item.is_none())
            };
            assert!(sent(&rx).iter().any(emptied), "the cleared result is sent");
        });
    });
}

#[test]
#[ignore = "daily"]
fn a_crafting_table_and_a_smithing_table_result_are_cleared() {
    with_lock(|| {
        with_game(|game| {
            let _rx = stick_guest(game);
            let player = game.player(123).unwrap();
            let stick = mod_api::__private::item_key("minecraft:stick").unwrap();
            let stack = |item: &str| Some(ItemStack::new(item, 1));
            KEEP.store(false, Ordering::Relaxed);
            *ANSWER.lock().unwrap() = None;
            heard();

            let crafting = menu_key("minecraft:crafting");
            let grid = vec![stack("minecraft:oak_log")];
            let window = game
                .player_open_station(player, crafting, grid, vec![stick])
                .unwrap()
                .unwrap();
            let menus = game.lan.as_ref().unwrap().menus.get(&1).unwrap();
            let (_, _, table, _) = menus.crafting.table.as_ref().unwrap();
            assert_eq!(table.slot(0), None);
            let planks = Some("minecraft:oak_planks".to_owned());
            assert_eq!(heard(), [(true, 9, planks)]);
            // A pickup and a shift-click on the cleared result take nothing and use nothing up.
            for input in [0, 1] {
                game.lan_guest_packet(
                    1,
                    Serverbound::ContainerClick(ContainerClick {
                        container: window,
                        state: 0,
                        slot: 0,
                        button: 0,
                        input,
                        changed: Vec::new(),
                        carried: None,
                    }),
                );
                let host = game.lan.as_ref().unwrap();
                let menus = host.menus.get(&1).unwrap();
                let (_, _, table, _) = menus.crafting.table.as_ref().unwrap();
                let grid = table.slot(1).map(|s| s.count);
                assert_eq!((table.slot(0), grid), (None, Some(1)), "input {input}");
                let guest = host.hub.guest(1).unwrap();
                assert_eq!(guest.carried, None, "input {input}");
                let no_planks = |s: &ItemStack| &*s.item != "minecraft:oak_planks";
                assert!(
                    guest.inventory.iter().flatten().all(no_planks),
                    "input {input}"
                );
            }
            assert_eq!(heard(), []);

            let smithing = menu_key("minecraft:smithing");
            let inputs = vec![
                stack("minecraft:netherite_upgrade_smithing_template"),
                stack("minecraft:diamond_sword"),
                stack("minecraft:netherite_ingot"),
            ];
            game.player_open_station(player, smithing, inputs, vec![stick])
                .unwrap()
                .unwrap();
            let menus = game.lan.as_ref().unwrap().menus.get(&1).unwrap();
            assert_eq!(menus.station.as_ref().unwrap().menu.slot(3), None);
            let sword = Some("minecraft:netherite_sword".to_owned());
            assert_eq!(heard(), [(true, 3, sword)]);
        });
    });
}
