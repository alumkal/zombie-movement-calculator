# 僵尸坐标分布计算器

不同于其他计算方法，本工具可以计算出僵尸坐标的**概率分布**（PvZ 内的坐标判定都使用取整后坐标，因此本工具给出的也是取整后坐标的分布）。除 PvZ 内部浮点误差只能尝试估计之外严格准确，可靠性高于流传的各种坐标表格。

## 使用方法

Windows 可执行文件可以在 [Releases](https://github.com/alumkal/zombie-movement-calculator/releases) 中获取。

程序内有输入提示。补充一点，“僵尸类型”是僵尸在英文原版图鉴中的名字，去掉后缀`Zombie` 和空格，比如撑杆就是 `PoleVaulting`（大小写无所谓）。例外：普僵是`Regular`、红眼可以是 `Giga` 或 `GigaGargantuar`。不知道英文名可以去 [pt](https://pvz.tools/wiki/#%E5%83%B5%E5%B0%B8-1) 查。 

此外，该计算器支持 [Dance秘籍](https://tieba.baidu.com/p/7921781826) 相关计算（对应的僵尸名是 `DCFast` 和 `DCSlow`）。概率 $<10^{-9}$ 的部分会被自动舍去。

示例：查询 200cs 炸9列收掉红眼的比例

输入：

```plain
giga
（直接回车）  
200  
0 817  
```

结果为 0.9599。

## 编译方法

安装 [Rust 套件](https://rustup.rs)，在本文件夹根目录下 `cargo build`。

## License

Copyright 2023 AlumKal

Licensed under the Apache License, Version 2.0 (the "License");
You may obtain a copy of the License at

<http://www.apache.org/licenses/LICENSE-2.0>

Unless required by applicable law or agreed to in writing, software
distributed under the License is distributed on an "AS IS" BASIS,
WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
See the License for the specific language governing permissions and
limitations under the License.

