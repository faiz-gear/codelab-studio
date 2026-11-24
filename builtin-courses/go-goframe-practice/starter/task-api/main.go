package main

import (
	"encoding/json"
	"log"
	"net/http"
	"time"
)

func routes() *http.ServeMux {
	mux := http.NewServeMux()
	mux.HandleFunc("GET /healthz", func(w http.ResponseWriter, _ *http.Request) {
		w.Header().Set("Content-Type", "application/json; charset=utf-8")
		if err := json.NewEncoder(w).Encode(map[string]string{"status": "ok"}); err != nil {
			log.Printf("write health response: %v", err)
		}
	})
	// 练习 2：在这里注册任务路由，并将请求交给共享的内存 Store。
	return mux
}

func main() {
	server := &http.Server{
		Addr:              "127.0.0.1:8080",
		Handler:           routes(),
		ReadHeaderTimeout: 5 * time.Second,
	}
	log.Printf("listening on http://%s", server.Addr)
	log.Fatal(server.ListenAndServe())
}
