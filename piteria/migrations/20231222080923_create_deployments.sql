-- Add migration script here
CREATE TABLE IF NOT EXISTS deployments (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL
);