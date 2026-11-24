package main

import (
	"github.com/gogf/gf/v2/frame/g"
	"github.com/gogf/gf/v2/net/ghttp"
)

func configure(server *ghttp.Server) {
	server.SetAddr("127.0.0.1:8000")
	server.BindHandler("GET:/tasks", listTasks)
}

func listTasks(request *ghttp.Request) {
	// 练习 7：调用已经完成的业务逻辑，返回真实的任务列表。
	request.Response.WriteJson([]any{})
}

func main() {
	server := g.Server()
	configure(server)
	server.Run()
}
