CREATE TABLE folders (
	id serial PRIMARY KEY,
	"path" varchar NOT NULL,
	updated_at timestamp(6) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
	created_at timestamp(6) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
	UNIQUE ("path")
);
ALTER TABLE bookmarks
ADD COLUMN folder_id integer REFERENCES folders(id);
