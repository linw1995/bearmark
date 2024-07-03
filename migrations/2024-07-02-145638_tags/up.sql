CREATE TABLE tags(
    id serial PRIMARY KEY,
    "name" varchar NOT NULL,
    updated_at timestamp(6) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    created_at timestamp(6) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE ("name")
);

CREATE TABLE bookmarks_tags(
    bookmark_id integer REFERENCES bookmarks(id) ON DELETE CASCADE,
    tag_id integer REFERENCES tags(id) ON DELETE CASCADE,
    PRIMARY KEY (bookmark_id, tag_id)
);

