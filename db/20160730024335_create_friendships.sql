-- Create friendships table
CREATE TABLE friendships (
    id BIGSERIAL PRIMARY KEY,
    person_id BIGINT REFERENCES people(id) ON DELETE CASCADE,
    friend_id BIGINT REFERENCES people(id) ON DELETE CASCADE,
    inserted_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITHOUT TIME ZONE NOT NULL DEFAULT NOW()
);

CREATE INDEX index_friendships_on_person_id ON friendships(person_id);
CREATE INDEX index_friendships_on_friend_id ON friendships(friend_id);
