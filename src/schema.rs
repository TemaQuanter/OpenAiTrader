// @generated automatically by Diesel CLI.

diesel::table! {
    channel_history (message_id) {
        message_id -> Int4,
        response_message_id -> Nullable<Int4>,
        message_text -> Nullable<Text>,
        message_timestamp -> Timestamptz,
    }
}

diesel::table! {
    role (role_id) {
        role_id -> Int4,
        title -> Varchar,
        role_description -> Text,
    }
}

diesel::table! {
    status (status_id) {
        status_id -> Int4,
        title -> Varchar,
        status_description -> Text,
    }
}

diesel::table! {
    users (telegram_id) {
        telegram_id -> Int8,
        telegram_username -> Nullable<Varchar>,
        status_id -> Int4,
        exchange_api_key -> Nullable<Varchar>,
        exchange_api_secret -> Nullable<Varchar>,
        exchange_api_data_validated -> Bool,
        deposit -> Nullable<Numeric>,
        risk -> Nullable<Numeric>,
        bot_active -> Bool,
        role_id -> Int4,
    }
}

diesel::joinable!(users -> role (role_id));
diesel::joinable!(users -> status (status_id));

diesel::allow_tables_to_appear_in_same_query!(
    channel_history,
    role,
    status,
    users,
);
