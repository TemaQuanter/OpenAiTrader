use trading_bot::handlers::bot::{dto_perpetual, dto_spot};

fn main() {
    let take_profit_signal = serde_json::from_str::<dto_perpetual::PositionOpening>(
        &"{
    \"ticker\": \"XRPUSDT\",\n
    \"direction\": \"long\",\n
    \"order_type\": \"market\",\n
    \"lower_bound_entry\": 1,
    \"upper_bound_entry\": 1.2,
    \"stop_loss\": 0.9,
    \"take_profit_lower_bound\": 4,
    \"take_profit_upper_bound\": 5
}",
    );

    println!("{:#?}", take_profit_signal);
}
