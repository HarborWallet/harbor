CREATE TABLE profile (
    id CHAR(36) PRIMARY KEY NOT NULL,
    seed_words CHAR(255) NOT NULL
);

CREATE TABLE fedimint (
    id CHAR(36) PRIMARY KEY NOT NULL,
    value BLOB NOT NULL
);
