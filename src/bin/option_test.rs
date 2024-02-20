use trading_bot::handlers::bot::dto_spot;

fn main() {
    let take_profit_signal = serde_json::from_str::<dto_spot::TakeProfitPlacement>(
        &"{
    \"ticker\": \"GRTUSDT\",
    \"take_profit_price\": 0.5,
    \"take_profit_percentage\": 100
}",
    );

    println!("{:#?}", take_profit_signal);
}
