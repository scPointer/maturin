initSidebarItems({"fn":[["check_virt_dir_exists","检查是否存在对应目录"],["check_virt_file_exists","检查是否存在对应文件。Some表示路径存在，true/false表示文件是否存在"],["get_virt_dir_if_possible","查询这个目录是否是 vfs 里的目录，是则返回对应目录"],["get_virt_file_if_possible","查询这个目录是否是 vfs 里的目录，如果是则从 vfs 中取对应文件"],["try_make_virt_dir","尝试新建目录，如果成功则创建这个目录，并存入 VFS_DIRS 中"],["try_remove_virt_file","删除对应文件或目录。Some表示路径存在，true/false表示文件是否存在"]],"mod":[["null","空文件，用于 dev/null"],["temp","临时性文件，语义上来说，OS可以定期清理其中文件，不保证永久保存"],["virt_dir","虚拟文件系统的目录。不需要考虑把数据塞进页里"],["virt_file","内存中保存的虚文件，可以有 backend 指向它实际映射的内容"],["zero","另一种空文件，用于 dev/zero"]],"struct":[["VFS_DIRS","属于虚拟文件系统的目录"]],"type":[["BufferFile",""]]});