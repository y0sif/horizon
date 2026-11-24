//! `menu_slot_pickup` through a real `MenuPart` on the read-only `Host`: a slot the part refuses
//! keeps its stack under every kind of click, in the window's own menu and in the LAN hub's,
//! the player's own inventory menu included. The fixtures need prepared resources.
use crate::game::native_entity_refs::tests::with_game;
use crate::game::native_guest_turn_tests::with_guests;
use horizon_client::{
    items::ItemStack,
    menu::{Input, Kind, Menu},
};
use mod_api::{
    Hooks, MenuContainer, MenuPart, MenuSlotPickupEvent, MenuSlotRulesEvent, ModEntry, Mods,
    Registrar,
};
use std::cell::{Cell, RefCell};

thread_local! {
    /// The slot the part refuses to give up: its container and backing index.
    static REFUSED: Cell<(MenuContainer, usize)> = const { Cell::new((MenuContainer::Main, 0)) };
    /// Every menu `menu_slot_rules` was asked for.
    static RULES_ASKED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

struct Lock;
#[mod_api::horizon_part]
impl MenuPart for Lock {
    fn menu_slot_pickup(&self, _: &dyn mod_api::Host, event: &mut MenuSlotPickupEvent<'_>) {
        assert!(
            event.may_pickup,
            "a slot that refuses on its own is not asked"
        );
        if (event.container, event.slot) == REFUSED.get() {
            event.may_pickup = false;
        }
    }
    fn menu_slot_rules(&self, _: &dyn mod_api::Host, event: &mut MenuSlotRulesEvent<'_>) {
        RULES_ASKED.with(|asked| asked.borrow_mut().push(event.menu.to_owned()));
    }
}
struct Listener;
#[mod_api::__subscriptions]
impl Hooks for Listener {
    fn register(&self, r: &mut dyn Registrar, _: &mod_api::RegistrationContext) {
        r.menu_part("minecraft:generic_9x3", Lock);
        r.menu_part("minecraft:furnace", Lock);
        r.menu_part("horizon:player_inventory", Lock);
    }
}

fn with_lock(test: impl FnOnce()) {
    let mods: Mods = vec![ModEntry::new("lock", Box::new(Listener))].into();
    crate::game::api_values::install_parts(&mods).unwrap();
    mod_platform::testing::with_mods(mods, test);
}

fn count(stack: Option<&ItemStack>) -> u32 {
    stack.map_or(0, |stack| stack.count)
}

#[test]
#[ignore = "daily"]
fn a_refused_slot_keeps_its_stack_under_every_click() {
    with_lock(|| {
        with_game(|game| {
            let env = game.menu_env();
            REFUSED.set((MenuContainer::Main, 0));
            let mut menu = Menu::new(Kind::Chest(3));
            menu.set_registry_id(Some("minecraft:generic_9x3"));
            menu.set_slot(&env, 0, Some(ItemStack::new("minecraft:diamond", 4)));
            menu.set_slot(&env, 1, Some(ItemStack::new("minecraft:diamond", 4)));
            // A pickup, a shift-click, a number-key swap and a throw all leave slot 0 alone.
            for (button, input) in [
                (0, Input::Pickup),
                (0, Input::QuickMove),
                (0, Input::Swap),
                (1, Input::Throw),
            ] {
                let _ = menu.click(&env, 0, button, input);
                assert_eq!(count(menu.slot(0)), 4, "{input:?}");
                assert!(menu.carried.is_none(), "{input:?}");
            }
            // A double-click collect skips the refused slot and takes the other one.
            menu.carried = Some(ItemStack::new("minecraft:diamond", 1));
            menu.click(&env, 5, 0, Input::PickupAll).unwrap();
            assert_eq!(
                (
                    count(menu.slot(0)),
                    count(menu.slot(1)),
                    count(menu.carried.as_ref())
                ),
                (4, 0, 5)
            );
            // A slot the part does not refuse gives its stack up as ever.
            REFUSED.set((MenuContainer::Main, 9));
            menu.carried = None;
            menu.click(&env, 0, 0, Input::Pickup).unwrap();
            assert_eq!((count(menu.slot(0)), count(menu.carried.as_ref())), (0, 4));

            // A station's result slot: a furnace's output (its slot 2) stays where it is.
            REFUSED.set((MenuContainer::Main, 2));
            let mut furnace = Menu::new(Kind::Furnace("smelting"));
            furnace.set_registry_id(Some("minecraft:furnace"));
            furnace.set_slot(&env, 2, Some(ItemStack::new("minecraft:iron_ingot", 3)));
            for input in [Input::Pickup, Input::QuickMove] {
                let _ = furnace.click(&env, 2, 0, input);
                assert_eq!(count(furnace.slot(2)), 3, "{input:?}");
                assert!(furnace.carried.is_none(), "{input:?}");
            }

            // The player's own inventory menu has no registered type: its part is the one on
            // `horizon:player_inventory`. A log in the grid keeps the result current.
            let mut inventory = Menu::new(Kind::Inventory);
            inventory.set_slot(&env, 1, Some(ItemStack::new("minecraft:oak_log", 1)));
            inventory.set_slot(&env, 5, Some(ItemStack::new("minecraft:iron_helmet", 1)));
            inventory.set_slot(&env, 9, Some(ItemStack::new("minecraft:diamond", 4)));
            for (row, slot, refused, held) in [
                ("result", 0, (MenuContainer::Result, 0), 4),
                ("grid", 1, (MenuContainer::Grid, 0), 1),
                ("helmet", 5, (MenuContainer::Player, 39), 1),
                ("main", 9, (MenuContainer::Player, 9), 4),
            ] {
                REFUSED.set(refused);
                for input in [Input::Pickup, Input::QuickMove] {
                    let _ = inventory.click(&env, slot as i32, 0, input);
                    assert_eq!(count(inventory.slot(slot)), held, "{row} {input:?}");
                    assert!(inventory.carried.is_none(), "{row} {input:?}");
                }
            }
            // An inventory slot the part does not refuse is taken and put back as ever.
            REFUSED.set((MenuContainer::Main, 0));
            inventory.click(&env, 9, 0, Input::Pickup).unwrap();
            assert_eq!(
                (count(inventory.slot(9)), count(inventory.carried.as_ref())),
                (0, 4)
            );
            inventory.click(&env, 9, 0, Input::Pickup).unwrap();
            assert_eq!(count(inventory.slot(9)), 4);
            // `menu_slot_rules` keeps its menu types: putting a stack down asks it for the chest,
            // never for the inventory menu.
            menu.click(&env, 0, 0, Input::Pickup).unwrap();
            let asked = RULES_ASKED.with(|asked| asked.borrow().clone());
            assert!(asked.iter().any(|menu| menu == "minecraft:generic_9x3"));
            assert!(!asked.iter().any(|menu| menu == "horizon:player_inventory"));
        });
    });
}

#[test]
#[ignore = "daily"]
fn a_lan_hub_click_leaves_a_refused_slot_in_the_chest() {
    use horizon_client::lan::menu::Slots;
    use horizon_protocol::packets::play::{ContainerClick, Serverbound};
    with_lock(|| {
        with_guests(|game| {
            // The guest's own inventory menu (window 0): the refused slot keeps its stack, and
            // gives it up once the part no longer refuses it.
            REFUSED.set((MenuContainer::Player, 9));
            let host = game.lan.as_mut().unwrap();
            host.hub.guests.update(0, |guest| {
                guest.inventory[9] = Some(ItemStack::new("minecraft:diamond", 4))
            });
            let own = |game: &mut crate::game::Game<'_>| {
                game.lan_guest_packet(
                    12,
                    Serverbound::ContainerClick(ContainerClick {
                        container: 0,
                        state: 0,
                        slot: 9,
                        button: 0,
                        input: 0,
                        changed: Vec::new(),
                        carried: None,
                    }),
                );
                let guest = &game.lan.as_ref().unwrap().hub.guests[0];
                (
                    count(guest.inventory[9].as_ref()),
                    count(guest.carried.as_ref()),
                )
            };
            assert_eq!(own(game), (4, 0));
            REFUSED.set((MenuContainer::Main, 0));
            assert_eq!(own(game), (0, 4));
            assert_eq!(own(game), (4, 0));

            let pos = (1, 64, 1);
            game.world.chunks.insert(
                (0, 0),
                std::sync::Arc::new(horizon_client::chunk::Chunk::empty(0, 0)),
            );
            let stone = game
                .pack
                .assets
                .registry
                .default_state("minecraft:stone")
                .unwrap();
            game.world.set(pos.0, pos.1, pos.2, stone).unwrap();
            let mut slots = vec![None; 27];
            slots[0] = Some(ItemStack::new("minecraft:diamond", 4));
            slots[1] = Some(ItemStack::new("minecraft:diamond", 4));
            game.world.block_entities.insert(
                pos,
                horizon_client::containers::BlockEntity::Chest(slots.clone()),
            );
            let host = game.lan.as_mut().unwrap();
            host.hub.open_container(
                12,
                "minecraft:generic_9x3",
                horizon_core::nbt::Tag::String("test".into()),
                pos,
                Slots::Block(pos),
                slots,
                None,
            );
            let window = host.hub.guests[0].menu.as_ref().unwrap().window;
            let click = |game: &mut crate::game::Game<'_>, slot: i16, input: i32| {
                game.lan_guest_packet(
                    12,
                    Serverbound::ContainerClick(ContainerClick {
                        container: window,
                        state: 0,
                        slot,
                        button: 0,
                        input,
                        changed: Vec::new(),
                        carried: None,
                    }),
                );
                let chest = game.world.block_entities.get(&pos).unwrap().slots();
                let carried = game.lan.as_ref().unwrap().hub.guests[0].carried.clone();
                (
                    count(chest[0].as_ref()),
                    count(chest[1].as_ref()),
                    count(carried.as_ref()),
                )
            };
            // PICKUP and QUICK_MOVE on the refused slot move nothing; slot 1 is taken as ever.
            assert_eq!(click(game, 0, 0), (4, 4, 0));
            assert_eq!(click(game, 0, 1), (4, 4, 0));
            assert_eq!(click(game, 1, 0), (4, 0, 4));
        });
    });
}
