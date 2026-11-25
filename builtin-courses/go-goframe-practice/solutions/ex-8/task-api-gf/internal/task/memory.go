package task

import (
	"context"
	"sort"
	"sync"
)

type Memory struct {
	mu     sync.RWMutex
	nextID int64
	tasks  map[int64]Task
}

func NewMemory() *Memory {
	return &Memory{tasks: make(map[int64]Task)}
}

func (m *Memory) Create(_ context.Context, title string) (Task, error) {
	m.mu.Lock()
	defer m.mu.Unlock()
	m.nextID++
	task := Task{ID: m.nextID, Title: title, Status: Pending}
	m.tasks[task.ID] = task
	return task, nil
}

func (m *Memory) Get(_ context.Context, id int64) (Task, error) {
	m.mu.RLock()
	defer m.mu.RUnlock()
	task, ok := m.tasks[id]
	if !ok {
		return Task{}, ErrNotFound
	}
	return task, nil
}

func (m *Memory) List(_ context.Context) ([]Task, error) {
	m.mu.RLock()
	tasks := make([]Task, 0, len(m.tasks))
	for _, task := range m.tasks {
		tasks = append(tasks, task)
	}
	m.mu.RUnlock()
	sort.Slice(tasks, func(i, j int) bool { return tasks[i].ID < tasks[j].ID })
	return tasks, nil
}

func (m *Memory) Rename(_ context.Context, id int64, title string) (Task, error) {
	return m.update(id, func(task *Task) { task.Title = title })
}

func (m *Memory) Complete(_ context.Context, id int64) (Task, error) {
	return m.update(id, func(task *Task) { task.Status = Done })
}

func (m *Memory) update(id int64, change func(*Task)) (Task, error) {
	m.mu.Lock()
	defer m.mu.Unlock()
	task, ok := m.tasks[id]
	if !ok {
		return Task{}, ErrNotFound
	}
	change(&task)
	m.tasks[id] = task
	return task, nil
}

func (m *Memory) Delete(_ context.Context, id int64) error {
	m.mu.Lock()
	defer m.mu.Unlock()
	if _, ok := m.tasks[id]; !ok {
		return ErrNotFound
	}
	delete(m.tasks, id)
	return nil
}
