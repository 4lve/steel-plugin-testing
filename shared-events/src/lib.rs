use steel_api::AbiString;

#[stabby::stabby]
pub struct PlayerGreetedEvent {
    pub player_name: AbiString,
}

steel_api::event!(PlayerGreetedEvent, "shared:player_greeted");
