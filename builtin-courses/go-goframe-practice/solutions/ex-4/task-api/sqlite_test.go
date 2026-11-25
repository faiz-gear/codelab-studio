package main

import (
	"context"
	"path/filepath"
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
