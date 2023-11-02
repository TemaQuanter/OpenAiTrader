use rust_decimal_macros::dec;
use trading_bot::{
    handlers::{
        bot::dto::{PositionClosure, PositionOpening, StopLossAdjustment},
        exchange::exchange_service::ExchangeService,
    },
    utils::credential_security::CredentialSecurity,
};

#[tokio::main]
async fn main() {
    let key = CredentialSecurity::decode_env_var("EXCHANGE_API").expect("no API key found");
    let secret =
        CredentialSecurity::decode_env_var("EXCHANGE_SECRET").expect("no API secret found");

    let client = ExchangeService::get_client(key, secret).unwrap();

    // println!(
    //     "{}",
    //     ExchangeService::get_uta_usdt_balance(&client)
    //         .await
    //         .unwrap()
    // );

    // println!(
    //     "{:#?}",
    //     ExchangeService::get_all_opened_positions(&client)
    //         .await
    //         .unwrap()
    // );

    // println!(
    //     "{}",
    //     ExchangeService::get_instrument_price_on_perpetual_swaps(&client, "BTCUSDT")
    //         .await
    //         .unwrap()
    // );

    let position_opening = PositionOpening {
        ticker: "XRPUSDT".to_string(),
        direction: "short".to_string(),
        risk: dec!(1.0),
        order_type: "Market".to_string(),
        lower_bound_entry: dec!(0.3),
        upper_bound_entry: dec!(0.3),
        stop_loss: dec!(0.4),
        take_profit_lower_bound: dec!(0.2),
        take_profit_upper_bound: dec!(0.1),
    };

    let stop_loss_adjustment = StopLossAdjustment {
        ticker: "XRPUSDT".to_string(),
        direction: "short".to_string(),
        new_stop: dec!(0.6),
    };

    let position_closure = PositionClosure {
        ticker: "XRPUSDT".to_string(),
        direction: "short".to_string(),
        percentage_to_close: dec!(100.0),
    };

    // println!(
    //     "{}",
    //     ExchangeService::get_quantity_step_on_perpetual_swaps(&client, "SOLUSDT")
    //         .await
    //         .unwrap()
    // );

    // println!(
    //     "{:#?}",
    //     ExchangeService::get_all_opened_orders(&client)
    //         .await
    //         .unwrap()
    // );

    // ExchangeService::get_quantity_step_on_perpetual_swaps(&client, "ETHUSDT")
    //     .await
    //     .unwrap();

    // ExchangeService::open_position_on_perpetual_swaps(&position_opening, &client)
    //     .await
    //     .unwrap();

    // ExchangeService::adjust_stop_loss_on_perpetual_swaps(&stop_loss_adjustment, &client)
    //     .await
    //     .unwrap();

    // ExchangeService::close_part_of_position_on_perpetual_swaps(&position_closure, &client)
    //     .await
    //     .unwrap();

    // public GET
    // let ticker: serde_json::Value = client
    //     .get(
    //         "/v5/market/tickers",
    //         Some(&[("category", "spot"), ("symbol", "BTCUSDT")]),
    //         [BybitOption::Default],
    //     )
    //     .await
    //     .expect("failed to get ticker");
    // println!("Ticker:\n{ticker}");
}
