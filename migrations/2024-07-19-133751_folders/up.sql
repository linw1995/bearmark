CREATE TABLE folders (
	id serial PRIMARY KEY,
	"path" varchar NOT NULL,
	updated_at timestamp(6) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
	created_at timestamp(6) with time zone NOT NULL DEFAULT CURRENT_TIMESTAMP,
	UNIQUE ("path")
);
CREATE TABLE bookmarks_folders(
	bookmark_id integer REFERENCES bookmarks(id) ON DELETE CASCADE,
	folder_id integer REFERENCES folders(id),
	PRIMARY KEY (bookmark_id, folder_id)
);
