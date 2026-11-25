package task

import (
	"context"
	"errors"
	"strings"
	"sync"
	"testing"
)

func TestTitleRules(t *testing.T) {
	service := NewService(NewMemory())
	ctx := context.Background()
	for _, title := range []string{"  ", strings.Repeat("学", 101)} {
		if _, err := service.Create(ctx, title); !errors.Is(err, ErrTitle) {
			t.Fatalf("invalid title accepted: %q (%v)", title, err)
		}
	}
	title := strings.Repeat("学", 100)
	created, err := service.Create(ctx, " "+title+" ")
	if err != nil || created.Title != title {
		t.Fatalf("valid title rejected or not trimmed: %+v, %v", created, err)
	}
}

func TestCompleteIsIdempotent(t *testing.T) {
	service := NewService(NewMemory())
	ctx := context.Background()
	if _, err := service.Get(ctx, 1); !errors.Is(err, ErrNotFound) {
		t.Fatalf("missing task: %v", err)
	}
	created, err := service.Create(ctx, "exercise")
	if err != nil {
		t.Fatal(err)
	}
	for range 2 {
		completed, err := service.Complete(ctx, created.ID)
		if err != nil || completed.Status != Done {
			t.Fatalf("complete: %+v, %v", completed, err)
		}
	}
}

func TestConcurrentCreatesAssignDistinctIDs(t *testing.T) {
	service := NewService(NewMemory())
	ctx := context.Background()
	var workers sync.WaitGroup
	for range 32 {
		workers.Add(1)
		go func() {
			defer workers.Done()
			if _, err := service.Create(ctx, "exercise"); err != nil {
				t.Error(err)
			}
		}()
	}
	workers.Wait()
	tasks, err := service.List(ctx)
	if err != nil || len(tasks) != 32 {
		t.Fatalf("list: %v, %v", tasks, err)
	}
	for i, item := range tasks {
		if item.ID != int64(i+1) {
			t.Fatalf("task %d has ID %d", i, item.ID)
		}
	}
}
