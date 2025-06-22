use std::sync::Mutex;

use rand::{
    rng,
    seq::{IndexedRandom, SliceRandom},
};
use rocket::{
    State,
    http::{ContentType, Status},
    serde::json::Json,
};
use serde::{Deserialize, Serialize};
use serde_json::to_string;
use strum::{EnumIter, IntoEnumIterator};

#[macro_use]
extern crate rocket;

#[launch]
fn rocket() -> _ {
    rocket::build()
        .mount(
            "/",
            routes![
                create_player,
                delete_player,
                get_players,
                get_player,
                create_game,
                delete_game,
                suggest
            ],
        )
        .manage(Mutex::new(GameState::new()))
    // .manage(Won { 0: -1 })
}

#[derive(Debug, Serialize)]
struct GameState {
    players: Vec<Player>,
    solution: Option<Suggestion>,
}

impl GameState {
    fn new() -> Self {
        GameState {
            players: Vec::new(),
            solution: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Suggestion {
    suspect: Suspect,
    weapon: Weapon,
    room: Room,
}

#[derive(Debug, Clone, Serialize)]
struct Player {
    name: String,
    cards: Vec<Card>,
}

impl Player {
    fn new(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            cards: Vec::<Card>::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
enum Card {
    Suspect(Suspect),
    Weapon(Weapon),
    Room(Room),
}

#[derive(Debug, Clone, EnumIter, PartialEq, Serialize, Deserialize)]
enum Suspect {
    Plum,
    Green,
    Mustard,
    Peacock,
    Scarlett,
    Orchid,
}

#[derive(Debug, Clone, EnumIter, PartialEq, Serialize, Deserialize)]
enum Weapon {
    Candlestick,
    LeadPipe,
    Dagger,
    Rope,
    Revolver,
    Wrench,
}

#[derive(Debug, Clone, EnumIter, PartialEq, Serialize, Deserialize)]
enum Room {
    Kitchen,
    Hall,
    Lounge,
    Ballroom,
    Conservatory,
    DiningRoom,
    Library,
    BilliardRoom,
    Study,
}

// struct Won(i8);

#[post("/players/<name>")]
fn create_player(name: &str, game_state: &State<Mutex<GameState>>) -> Status {
    let mut state = game_state.lock().expect("Failed to lock GameState");

    match state.players.iter().find(|p| p.name == name.to_owned()) {
        Some(_) => Status::Conflict,
        None => {
            state.players.push(Player::new(name));
            Status::Created
        }
    }
}

#[delete("/players/<name>")]
fn delete_player(name: &str, game_state: &State<Mutex<GameState>>) -> Status {
    let mut state = game_state.lock().expect("Failed to lock GameState");

    match state.players.iter().position(|p| p.name == name.to_owned()) {
        Some(index) => {
            state.players.remove(index);
            Status::NoContent
        }
        None => Status::NotFound,
    }
}

#[get("/players")]
fn get_players(game_state: &State<Mutex<GameState>>) -> (ContentType, String) {
    let players = &game_state.lock().expect("Failed to lock GameState").players;

    (ContentType::JSON, to_string(&players).unwrap())
}

#[get("/players/<name>")]
fn get_player(
    name: &str,
    game_state: &State<Mutex<GameState>>,
) -> Result<(ContentType, String), Status> {
    let players = &game_state.lock().expect("Failed to lock members").players;

    if let Some(index) = players.iter().position(|p| p.name == name.to_owned()) {
        Ok((ContentType::JSON, to_string(&players[index]).unwrap()))
    } else {
        Err(Status::NotFound)
    }
}

#[post("/game")]
fn create_game(
    game_state: &State<Mutex<GameState>>,
) -> Result<(Status, (ContentType, String)), Status> {
    let mut state = game_state.lock().expect("Failed to lock solution");

    match state.solution {
        Some(_) => (),
        None => return Err(Status::BadRequest),
    }

    if state.players.len() < 2 {
        return Err(Status::BadRequest);
    }

    let mut rng = rng();

    let suspects: Vec<Suspect> = Suspect::iter().collect();
    let weapons: Vec<Weapon> = Weapon::iter().collect();
    let rooms: Vec<Room> = Room::iter().collect();

    let murder_suspect = suspects.choose(&mut rng).unwrap();
    let murder_weapon = weapons.choose(&mut rng).unwrap();
    let murder_room = rooms.choose(&mut rng).unwrap();

    state.solution = Some(Suggestion {
        suspect: murder_suspect.clone(),
        weapon: murder_weapon.clone(),
        room: murder_room.clone(),
    });

    let mut all_cards = Vec::new();

    all_cards.extend(Suspect::iter().map(Card::Suspect));
    all_cards.extend(Weapon::iter().map(Card::Weapon));
    all_cards.extend(Room::iter().map(Card::Room));

    let solution_vector: Vec<Card> = vec![
        Card::Suspect(murder_suspect.to_owned()),
        Card::Weapon(murder_weapon.to_owned()),
        Card::Room(murder_room.to_owned()),
    ];
    all_cards.retain(|x| !solution_vector.contains(x));

    all_cards.shuffle(&mut rng);

    let total_cards = all_cards.len();
    let num_players = state.players.len();

    let base_cards_per_player = total_cards / num_players;
    let extra_cards = total_cards % num_players;

    let mut card_index = 0;

    for _ in 0..base_cards_per_player {
        for player_index in 0..num_players {
            if card_index < all_cards.len() {
                state.players[player_index]
                    .cards
                    .push(all_cards[card_index].clone());
                card_index += 1;
            }
        }
    }

    for player_index in 0..extra_cards {
        if card_index < all_cards.len() {
            state.players[player_index]
                .cards
                .push(all_cards[card_index].clone());
            card_index += 1;
        }
    }

    Ok((
        Status::Created,
        (ContentType::JSON, to_string(&*state).unwrap()),
    ))
}

#[delete("/game")]
fn delete_game(game_state: &State<Mutex<GameState>>) -> Status {
    let mut state = game_state.lock().expect("Failed to lock solution");

    match state.solution {
        Some(_) => (),
        None => return Status::BadRequest,
    }

    state.players = Vec::new();
    state.solution = None;

    Status::NoContent
}

#[post("/suggest", data = "<suggestion>")]
fn suggest(suggestion: Json<Suggestion>, game_state: &State<Mutex<GameState>>) -> Status {
    let state = &game_state.lock().expect("Failed to lock GameState");

    for player in &state.players {
        player.cards.iter().find(|c| {
            *c == &Card::Suspect(suggestion.0.suspect.clone())
                || *c == &Card::Weapon(suggestion.0.weapon.clone())
                || *c == &Card::Room(suggestion.0.room.clone())
        });
    }

    todo!()
}
