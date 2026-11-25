package task

import (
	"context"
	"database/sql"
	"fmt"

	_ "github.com/gogf/gf/contrib/drivers/sqlite/v2"
	"github.com/gogf/gf/v2/database/gdb"
)

type ORM struct {
	db gdb.DB
}

func OpenORM(ctx context.Context, path string, migrations ...string) (*ORM, error) {
	db, err := gdb.New(gdb.ConfigNode{
		Type:             "sqlite",
		Name:             path,
		MaxOpenConnCount: 1,
	})
	if err != nil {
		return nil, err
	}
	for _, migration := range migrations {
		if _, err := db.Exec(ctx, migration); err != nil {
			_ = db.Close(ctx)
			return nil, fmt.Errorf("apply migration: %w", err)
		}
	}
	return &ORM{db: db}, nil
}

func (s *ORM) Close(ctx context.Context) error {
	return s.db.Close(ctx)
}

func (s *ORM) Create(ctx context.Context, title string) (Task, error) {
	id, err := s.db.Model("tasks").Ctx(ctx).Data(gdb.Map{"title": title}).InsertAndGetId()
	return Task{ID: id, Title: title, Status: Pending}, err
}

func (s *ORM) Get(ctx context.Context, id int64) (Task, error) {
	return getTask(s.db.Model("tasks").Ctx(ctx).Where("id", id))
}

func (s *ORM) List(ctx context.Context) ([]Task, error) {
	tasks := []Task{}
	err := s.db.Model("tasks").Ctx(ctx).Fields("id", "title", "status").OrderAsc("id").Scan(&tasks)
	if err == sql.ErrNoRows {
		return tasks, nil
	}
	return tasks, err
}

func (s *ORM) Rename(ctx context.Context, id int64, title string) (item Task, err error) {
	err = s.db.Transaction(ctx, func(ctx context.Context, tx gdb.TX) error {
		model := tx.Model("tasks").Ctx(ctx).Where("id", id)
		result, err := model.Clone().Data(gdb.Map{"title": title, "updated_at": gdb.Raw("CURRENT_TIMESTAMP")}).Update()
		if err != nil {
			return err
		}
		if err := requireRow(result); err != nil {
			return err
		}
		item, err = getTask(model)
		return err
	})
	return
}

func (s *ORM) Complete(ctx context.Context, id int64) (item Task, err error) {
	err = s.db.Transaction(ctx, func(ctx context.Context, tx gdb.TX) error {
		task := tx.Model("tasks").Ctx(ctx).Where("id", id)
		result, err := task.Clone().Where("status", Pending).Data(gdb.Map{"status": Done, "updated_at": gdb.Raw("CURRENT_TIMESTAMP")}).Update()
		if err != nil {
			return err
		}
		changed, err := result.RowsAffected()
		if err != nil {
			return err
		}
		if changed == 1 {
			if _, err := tx.Model("task_events").Ctx(ctx).Data(gdb.Map{"task_id": id, "kind": "completed"}).Insert(); err != nil {
				return fmt.Errorf("record completion: %w", err)
			}
		}
		item, err = getTask(task)
		return err
	})
	return
}

func (s *ORM) Delete(ctx context.Context, id int64) error {
	result, err := s.db.Model("tasks").Ctx(ctx).Where("id", id).Delete()
	if err != nil {
		return err
	}
	return requireRow(result)
}

func getTask(model *gdb.Model) (Task, error) {
	record, err := model.Fields("id", "title", "status").One()
	if err != nil {
		return Task{}, err
	}
	if record.IsEmpty() {
		return Task{}, ErrNotFound
	}
	var item Task
	err = record.Struct(&item)
	return item, err
}

func requireRow(result sql.Result) error {
	count, err := result.RowsAffected()
	if err == nil && count == 0 {
		return ErrNotFound
	}
	return err
}
