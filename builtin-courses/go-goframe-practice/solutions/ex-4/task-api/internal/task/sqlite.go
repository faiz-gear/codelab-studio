package task

import (
	"context"
	"database/sql"
	"errors"
	"fmt"

	_ "modernc.org/sqlite"
)

type SQLite struct {
	db *sql.DB
}

func OpenSQLite(ctx context.Context, path string, migrations ...string) (*SQLite, error) {
	db, err := sql.Open("sqlite", path)
	if err != nil {
		return nil, err
	}
	db.SetMaxOpenConns(1)
	if err := db.PingContext(ctx); err != nil {
		db.Close()
		return nil, fmt.Errorf("ping SQLite: %w", err)
	}
	for _, migration := range migrations {
		if _, err := db.ExecContext(ctx, migration); err != nil {
			db.Close()
			return nil, fmt.Errorf("apply migration: %w", err)
		}
	}
	return &SQLite{db: db}, nil
}

func (s *SQLite) Close() error {
	return s.db.Close()
}

func (s *SQLite) Create(ctx context.Context, title string) (Task, error) {
	result, err := s.db.ExecContext(ctx, "INSERT INTO tasks (title) VALUES (?)", title)
	if err != nil {
		return Task{}, err
	}
	id, err := result.LastInsertId()
	if err != nil {
		return Task{}, err
	}
	return Task{ID: id, Title: title, Status: Pending}, nil
}

func (s *SQLite) Get(ctx context.Context, id int64) (Task, error) {
	return scanTask(s.db.QueryRowContext(ctx, "SELECT id, title, status FROM tasks WHERE id = ?", id))
}

func (s *SQLite) List(ctx context.Context) ([]Task, error) {
	rows, err := s.db.QueryContext(ctx, "SELECT id, title, status FROM tasks ORDER BY id ASC")
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	tasks := []Task{}
	for rows.Next() {
		item, err := scanTask(rows)
		if err != nil {
			return nil, err
		}
		tasks = append(tasks, item)
	}
	return tasks, rows.Err()
}

func (s *SQLite) Rename(ctx context.Context, id int64, title string) (Task, error) {
	return scanTask(s.db.QueryRowContext(ctx, `UPDATE tasks SET title = ?, updated_at = CURRENT_TIMESTAMP
        WHERE id = ? RETURNING id, title, status`, title, id))
}

func (s *SQLite) Complete(ctx context.Context, id int64) (Task, error) {
	return scanTask(s.db.QueryRowContext(ctx, `UPDATE tasks SET status = 'done', updated_at = CURRENT_TIMESTAMP
        WHERE id = ? RETURNING id, title, status`, id))
}

func (s *SQLite) Delete(ctx context.Context, id int64) error {
	result, err := s.db.ExecContext(ctx, "DELETE FROM tasks WHERE id = ?", id)
	if err != nil {
		return err
	}
	rows, err := result.RowsAffected()
	if err != nil {
		return err
	}
	if rows == 0 {
		return ErrNotFound
	}
	return nil
}

func scanTask(row interface{ Scan(...any) error }) (Task, error) {
	var item Task
	err := row.Scan(&item.ID, &item.Title, &item.Status)
	if errors.Is(err, sql.ErrNoRows) {
		return Task{}, ErrNotFound
	}
	return item, err
}
