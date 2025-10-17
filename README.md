# Oracle Internal LABs

这个项目主要用于探索与分析Oracle Redo文件的结构，对于物理、逻辑结构的描述，
~~将使用 [010 Editor](https://www.sweetscape.com/010editor/) 的模板来进行描述。~~
（放弃了，有些不太好用这个实现，直接一步到位编程实现一个解析库。）

## 分析方法

1. 编写最小化的测试用例，生成Redo文件，并使用Oracle dump成文本并提取，配合十六进制编辑工具进行分析对比，确定Redo的物理结构。
2. 使用其他资料用来做交叉验证。
3. 修改redo文件，重新计算checksum，使用Oracle进行dump，比对结果。

## 其他说明

因为项目本身的性质，所以可能无法解释文件中的某些信息，在注释中将使用一些标记进行说明，因为不希望大段的话总是在注释中重复出现，所以使用缩写进行标记(使用`#{}`包裹起来)：

- AMND(After modified, and dump it, no any difference.): 将二进制文件修改后，重新dump，未发现结果变化（无法通过此方法确认字节的含义）
- AME(After modified, and dump it, error occurred.): 将二进制文件修改后，重新dump，数据库报错（修改的内容以及错误会具体进行说明）

## TODOs

- [ ] Replace `bitflags` with `bitfields` for better struct declaration and expression.
