# Redo Block

结合资料，将Redo Block的结构进行确认，主要是将 Change Vector 进行确认，确认的过程中，发现我实际拿到的 Redo 文件与资料中的描述多少有些出入，这可能是由于我用于分析的 Oracle DataBase 版本较新（21.3.0）。

## 参考资料

- [Oracle Scratchpad - Redo OP Codes](https://jonathanlewis.wordpress.com/2017/07/25/redo-op-codes/)
- [Oracle RedoLog-基本概念和组成 - chuonye](https://www.cnblogs.com/chuonye/p/14229283.html)
- [Oracle RedoLog-二进制格式分析，文件头，DML，DDL - chuonye](https://www.cnblogs.com/chuonye/p/14248895.html)
