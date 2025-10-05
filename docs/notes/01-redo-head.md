# Redo 基本结构确认

使用任意二进制编辑工具，结合导出的归档文件的dump结果来看，可以很快的将redo文件头部的大部分信息确认下来
部分信息稍微需要花一些时间来梳理（结合相关资料可以进一步加快速度），不过总体上来说，这部分确认起来还是很快的，具体可以自行对比。

这些分析主要涵盖了以下信息：

1. `file.header`
2. `file.redoHeader`

## 参考资料

- [Oracle RedoLog-二进制格式分析，文件头，DML，DDL - chuonye](https://www.cnblogs.com/chuonye/p/14248895.html)
- [Oracle redo解析之-1、oracle redo log结构计算](https://blog.csdn.net/weixin_34001430/article/details/91397563)
