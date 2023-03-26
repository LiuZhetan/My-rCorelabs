# My-rCorelabs
学习清华的rcore项目，用rust写一个运行在risc v架构上的操作系统内核

项目教程地址：http://rcore-os.cn/rCore-Tutorial-Book-v3/index.html

rcore github: https://github.com/rcore-os/rCore-Tutorial-v3

本项目主要以完成rcore的lab为主，也会参考教程的编程题对rcore进行拓展。
# 任务清单
1. ch1：跳过
2. ch2: 完成所有的课后练习编程题
3. ch3：完成课后练习编程题1、2、4、5
4. ch4: 重新实现了sys_get_time,参照《算法导论》用rust实现了红黑树并以此实现了区间树用于实现mmap和unmmap系统调用，最终有bug，待修复
5. ch5: 实现了spawn系统调用，实现了stride调度算法
以上截止2023.2.25
待完成：
ch6,ch7,ch8,ch9的实验练习，最后打算实现多核的rcore

2023.3.18更新
1. ch7-lab：（实际上是第六章的文件系统）在easy_fs中实现了link、remove_file、fallocate、fdeallocate功能
2. link：创建硬链接
3. remove_file：删除文件，删除硬链接的话会使得文件的链接计数-1
4. fallocate：类似于linux的fallocate调用，在文件中插入一段空洞（会分配存储空间）
5. fdeallocate: 在文件中删除一段空间，回收块


2023.3.26更新
目标：尝试实现多级目录
1. 在TCB中引入了工作目录，对fork，exec系统调用进行修改，fork使得子进程可以复制父进程的工作目录，exec可以修改当前进程的工作目录
2. 在内核完成了mkdir、chdir、getdents等目录的系统调用，但在用户库中还没有实现对应的库函数
3. bug待修复：创建第一个初试进程时由于找不到父进程报错
