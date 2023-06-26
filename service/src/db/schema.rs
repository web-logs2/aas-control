// @generated automatically by Diesel CLI.

diesel::table! {
    /// Representation of the `openanolis_users` table.
    ///
    /// (Automatically generated by Diesel.)
    openanolis_users (id) {
        /// ID
        id -> Bigint,
        /// no
        userno -> Text,
        /// user name
        username -> Text,
        /// user Email
        email -> Text,
        /// AAS instance auth private key
        aas_auth_key -> Nullable<Text>,
        /// The `aas_instance` column of the `openanolis_users` table.
        ///
        /// Its SQL type is `Bool`.
        ///
        /// (Automatically generated by Diesel.)
        aas_instance -> Bool,
        /// The `insert_time` column of the `openanolis_users` table.
        ///
        /// Its SQL type is `Timestamp`.
        ///
        /// (Automatically generated by Diesel.)
        insert_time -> Timestamp,
    }
}
