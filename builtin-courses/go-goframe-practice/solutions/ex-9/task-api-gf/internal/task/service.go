package task

import (
	"context"
	"errors"
	"strings"
	"unicode/utf8"
)

const (
	Pending = "pending"
	Done    = "done"
)

var (
	ErrNotFound = errors.New("task not found")
	ErrTitle    = errors.New("title must contain 1–100 Unicode code points")
)

type Task struct {
	ID     int64  `json:"id"`
	Title  string `json:"title"`
	Status string `json:"status"`
}

type Store interface {
	Create(context.Context, string) (Task, error)
	Get(context.Context, int64) (Task, error)
	List(context.Context) ([]Task, error)
	Rename(context.Context, int64, string) (Task, error)
	Complete(context.Context, int64) (Task, error)
	Delete(context.Context, int64) error
}

type Service struct {
	Store
}

func NewService(store Store) *Service {
	return &Service{Store: store}
}

func (s *Service) Create(ctx context.Context, title string) (Task, error) {
	title, err := normalizeTitle(title)
	if err != nil {
		return Task{}, err
	}
	return s.Store.Create(ctx, title)
}

func (s *Service) Rename(ctx context.Context, id int64, title string) (Task, error) {
	title, err := normalizeTitle(title)
	if err != nil {
		return Task{}, err
	}
	return s.Store.Rename(ctx, id, title)
}

func normalizeTitle(title string) (string, error) {
	title = strings.TrimSpace(title)
	if size := utf8.RuneCountInString(title); size < 1 || size > 100 {
		return "", ErrTitle
	}
	return title, nil
}
