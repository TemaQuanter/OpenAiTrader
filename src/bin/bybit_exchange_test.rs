use std::sync::Arc;

use rust_decimal_macros::dec;
use trading_bot::{
    db_objects::user_model::User,
    handlers::{
        bot::dto_perpetual,
        bot::dto_spot,
        exchange::{
            exchange_service::ExchangeService,
            exchange_service_perpetual::ExchangeServicePerpetual,
            exchange_service_spot::ExchangeServiceSpot,
        },
    },
    utils::{credential_security::CredentialSecurity, database_service::DatabaseService},
};

#[tokio::main]
async fn main() {
    // let key = CredentialSecurity::decode_env_var("EXCHANGE_API").expect("no API key found");
    // let secret =
    //     CredentialSecurity::decode_env_var("EXCHANGE_SECRET").expect("no API secret found");

    let test_user_telegram_id = CredentialSecurity::decode_env_var("TEST_USER_TG_ID")
        .expect("no test user tg id found")
        .parse::<i64>()
        .unwrap();

    // Create a database connection pool.
    let pg_connection_pool = DatabaseService::create_database_connection_pool();

    // Get database connection.
    let conn = &mut DatabaseService::get_pool_connection(&pg_connection_pool).unwrap();

    // let client = ExchangeServicePerpetual::get_client(key, secret);

    // Get a test user from the database.
    let test_user = User::get_from_db_by_id(test_user_telegram_id, conn).unwrap();

    let client = ExchangeService::get_client(
        test_user.exchange_api_key.clone().unwrap(),
        test_user.exchange_api_secret.clone().unwrap(),
    );

    // println!(
    //     "{}",
    //     ExchangeServicePerpetual::get_quantity_step(&client, "SUIUSDT".to_string())
    //         .await
    //         .unwrap()
    // );

    // println!(
    //     "{:#?}",
    //     ExchangeService::get_api_deadline_days(&client)
    //         .await
    //         .unwrap()
    // );

    // println!(
    //     "{:#?}",
    //     ExchangeServiceSpot::get_opened_position(&client, "USDT")
    //         .await
    //         .unwrap()
    // );

    // println!(
    //     "{:#?}",
    //     ExchangeServiceSpot::get_all_opened_orders(&client)
    //         .await
    //         .unwrap()
    // );

    // println!(
    //     "{:#?}",
    //     ExchangeServiceSpot::get_quantity_step(&client, "XRPUSDT")
    //         .await
    //         .unwrap()
    // );

    let position_opening = dto_spot::PositionOpening {
        ticker: "UNIUSDT".to_string(),
        percentage_from_deposit: dec!(2),
        order_type: "market".to_string(),
        lower_bound_entry: dec!(0.09),
        upper_bound_entry: dec!(0.15),
        stop_loss: None,
        take_profit_lower_bound: Some(dec!(0.27)),
        take_profit_upper_bound: Some(dec!(0.75)),
    };

    let position_closure = dto_spot::PositionClosure {
        ticker: "UNIUSDT".to_string(),
        percentage_to_close: dec!(100),
    };

    let position_increase = dto_spot::PositionIncrease {
        ticker: "UNIUSDT".to_string(),
        percentage_to_increase: dec!(1),
        new_stop: None,
        percentage_from: "deposit".to_string(),
    };

    let stop_loss_adjustment = dto_spot::StopLossAdjustment {
        ticker: "UNIUSDT".to_string(),
        new_stop: Some(dec!(6.5)),
    };

    let order_removal = dto_spot::ConditionalOrderRemoval {
        ticker: "UNIUSDT".to_string(),
        trigger_price: dec!(7),
    };

    // ExchangeServiceSpot::open_position(Arc::new(position_opening), test_user)
    //     .await
    //     .unwrap();

    // ExchangeServiceSpot::adjust_stop_loss(Arc::new(stop_loss_adjustment), test_user)
    //     .await
    //     .unwrap();

    // ExchangeServiceSpot::cancel_order(Arc::new(order_removal), test_user)
    //     .await
    //     .unwrap();

    // ExchangeServiceSpot::increase_position(Arc::new(position_increase), test_user)
    //     .await
    //     .unwrap();

    // ExchangeServiceSpot::close_part_of_position(Arc::new(position_closure), test_user)
    //     .await
    //     .unwrap();

    // ExchangeServicePerpetual::set_leverage(&client, &"XRPUSDT".to_string(), &"Buy".to_string(), dec!(8))
    //     .await
    //     .unwrap();

    // println!(
    //     "{:#?}",
    //     ExchangeServicePerpetual::get_position_leverage(&client, &"XRPUSDT".to_string())
    //         .await
    //         .unwrap()
    // );

    // println!(
    //     "{}",
    // ExchangeServicePerpetual::get_uta_total_equity(&client)
    //         .await
    //         .unwrap()
    // );

    // println!(
    //     "{}",
    //     ExchangeServicePerpetual::get_instrument_price_on_perpetual_swaps(&client, "1000LADYSUSDT")
    //         .await
    //         .unwrap()
    // );

    // println!(
    //     "{:#?}",
    //     ExchangeServicePerpetual::get_all_opened_positions(&client)
    //         .await
    //         .unwrap()
    // );

    println!(
        "{:#?}",
        ExchangeServiceSpot::_place_partial_take_profit_order(
            &client,
            &"UNIUSDT".to_string(),
            "8".to_string(),
            "10".to_string()
        )
        .await
    )

    // println!(
    //     "{:#?}",
    //     ExchangeServiceSpot::get_opened_position(&client, "SEILOR")
    //         .await
    //         .unwrap()
    // );

    // println!(
    //     "{}",
    //     ExchangeServicePerpetual::get_instrument_price_on_perpetual_swaps(&client, "BTCUSDT")
    //         .await
    //         .unwrap()
    // );

    // let position_opening = PositionOpening {
    //     ticker: "XRPUSDT".to_string(),
    //     direction: "short".to_string(),
    //     risk: dec!(1.0),
    //     order_type: "market".to_string(),
    //     lower_bound_entry: dec!(0.6),
    //     upper_bound_entry: dec!(0.7),
    //     stop_loss: dec!(0.75),
    //     take_profit_lower_bound: Some(dec!(0.5)),
    //     take_profit_upper_bound: Some(dec!(0.4)),
    // };

    // let stop_loss_adjustment = StopLossAdjustment {
    //     ticker: "XRPUSDT".to_string(),
    //     direction: "short".to_string(),
    //     new_stop: dec!(0.6),
    // };

    // let position_closure = PositionClosure {
    //     ticker: "XRPUSDT".to_string(),
    //     direction: "short".to_string(),
    //     percentage_to_close: dec!(100.0),
    // };

    // let position_increase = PositionIncrease {
    //     ticker: "XRPUSDT".to_string(),
    //     direction: "short".to_string(),
    //     new_stop: dec!(0.69),
    //     percentage_to_increase: dec!(100.0),
    // };

    // println!(
    //     "{}",
    //     ExchangeServicePerpetual::get_quantity_step_on_perpetual_swaps(&client, "SOLUSDT")
    //         .await
    //         .unwrap()
    // );

    // println!(
    //     "{:#?}",
    //     ExchangeServicePerpetual::get_all_opened_orders(&client)
    //         .await
    //         .unwrap()
    // );

    // ExchangeServicePerpetual::get_quantity_step_on_perpetual_swaps(&client, "ETHUSDT")
    //     .await
    //     .unwrap();

    // ExchangeServicePerpetual::open_position_on_perpetual_swaps(
    //     Arc::new(position_opening),
    //     test_user.clone(),
    // )
    // .await
    // .unwrap();

    // ExchangeServicePerpetual::increase_position_on_perpetual_swaps(
    //     Arc::new(position_increase),
    //     test_user.clone(),
    // )
    // .await
    // .unwrap();

    // ExchangeServicePerpetual::adjust_stop_loss_on_perpetual_swaps(&stop_loss_adjustment, &client)
    //     .await
    //     .unwrap();

    // ExchangeServicePerpetual::close_part_of_position_on_perpetual_swaps(&position_closure, &client)
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
