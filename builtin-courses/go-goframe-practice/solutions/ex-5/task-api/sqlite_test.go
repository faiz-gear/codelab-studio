package main

import (
	"context"
	"database/sql"
	"path/filepath"
	"strings"
	"sync"
	"testing"

	"example.com/task-api/internal/task"
)

func TestTasksSurviveReopeningSQLite(t *testing.T) {
	ctx := context.Background()
	filename := filepath.Join(t.TempDir(), "tasks.db")
	store, err := task.OpenSQLite(ctx, filename, createTasks)
	if err != nil {
		t.Fatal(err)
	}
	created, err := store.Create(ctx, "Learner's task")
	if err != nil {
		t.Fatal(err)
	}
	if err := store.Close(); err != nil {
		t.Fatal(err)
	}

	store, err = task.OpenSQLite(ctx, filename, createTasks)
	if err != nil {
		t.Fatal(err)
	}
	defer store.Close()
	tasks, err := store.List(ctx)
	if err != nil || len(tasks) != 1 || tasks[0] != created {
		t.Fatalf("persisted tasks: %v, %v", tasks, err)
	}
}

func TestCompletionRollsBackAndFiresOnlyOnce(t *testing.T) {
	ctx := context.Background()
	filename := filepath.Join(t.TempDir(), "tasks.db")
	admin, err := sql.Open("sqlite", filename)
	if err != nil {
		t.Fatal(err)
	}
	defer admin.Close()
	store, err := task.OpenSQLite(ctx, filename, createTasks, createEvents)
	if err != nil {
		t.Fatal(err)
	}
	defer store.Close()
	item, err := store.Create(ctx, "exercise")
	if err != nil {
		t.Fatal(err)
	}

	if _, err := admin.Exec(`CREATE TRIGGER prevent_event BEFORE INSERT ON task_events
        BEGIN SELECT RAISE(ABORT, 'forced event error'); END`); err != nil {
		t.Fatal(err)
	}
	if _, err := store.Complete(ctx, item.ID); err == nil || !strings.Contains(err.Error(), "forced event error") {
		t.Fatalf("expected forced event error, got %v", err)
	}
	pending, err := store.Get(ctx, item.ID)
	if err != nil || pending.Status != task.Pending {
		t.Fatalf("completion was not rolled back: %+v, %v", pending, err)
	}

	if _, err := admin.Exec("DROP TRIGGER prevent_event"); err != nil {
		t.Fatal(err)
	}
	var workers sync.WaitGroup
	for range 8 {
		workers.Add(1)
		go func() {
			defer workers.Done()
			done, err := store.Complete(ctx, item.ID)
			if err != nil || done.Status != task.Done {
				t.Errorf("complete: %+v, %v", done, err)
			}
		}()
	}
	workers.Wait()
	var count int
	if err := admin.QueryRow("SELECT count(*) FROM task_events WHERE task_id = ?", item.ID).Scan(&count); err != nil || count != 1 {
		t.Fatalf("event count = %d, %v", count, err)
	}
}
