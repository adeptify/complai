//! 内置 agent workflow skill 的发现与按需加载。
//!
//! 真正的 workflow 正文放在 `src/skills_content/` 并编译进二进制，使安装后的
//! `complai` 不依赖源码目录。顶层 `skills/complai/` 只保留供 agent 客户端安装的
//! discovery stub；二者分离可以避免把所有 workflow 常驻加载到上下文。
//! `list` 只给路由所需的紧凑摘要，`get` 才输出完整 prompt，避免把所有
//! 工作流同时塞进 agent 上下文。

use crate::cli::SkillCommand;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Skill {
    pub name: &'static str,
    pub description: &'static str,
    pub prompt: &'static str,
}

const SKILLS: &[Skill] = &[
    Skill {
        name: "project-init",
        description: "初始化系统与合规框架绑定的项目工作区",
        prompt: include_str!("skills_content/project-init/SKILL.md"),
    },
    Skill {
        name: "doc-ingest",
        description: "解析 Excel 等文档并灌入对应知识库",
        prompt: include_str!("skills_content/doc-ingest/SKILL.md"),
    },
    Skill {
        name: "gap-analysis",
        description: "逐控制项执行差距分析并生成报告",
        prompt: include_str!("skills_content/gap-analysis/SKILL.md"),
    },
];

pub fn run(command: SkillCommand) -> eyre::Result<()> {
    match command {
        SkillCommand::List => {
            list();
            Ok(())
        }
        SkillCommand::Get { skill_name } => get(&skill_name),
    }
}

/// 返回可供 agent 做首次路由的紧凑索引。
pub fn available() -> &'static [Skill] {
    SKILLS
}

fn list() {
    println!("NAME\tDESCRIPTION");
    for skill in available() {
        println!("{}\t{}", skill.name, skill.description);
    }
}

fn get(name: &str) -> eyre::Result<()> {
    let Some(skill) = available().iter().find(|skill| skill.name == name) else {
        let names = available()
            .iter()
            .map(|skill| skill.name)
            .collect::<Vec<_>>()
            .join(", ");
        return Err(eyre::eyre!("未知 skill `{name}`(可用: {names})"));
    };

    print!("{}", skill.prompt);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_names_match_embedded_frontmatter() {
        for skill in available() {
            assert!(
                skill
                    .prompt
                    .starts_with(&format!("---\nname: {}\n", skill.name)),
                "内置 skill `{}` 的注册名应与 frontmatter 一致",
                skill.name
            );
            assert!(skill.prompt.contains("\ndescription: "));
        }
    }

    #[test]
    fn registry_names_are_unique() {
        for (index, skill) in available().iter().enumerate() {
            assert!(
                available()[index + 1..]
                    .iter()
                    .all(|candidate| candidate.name != skill.name),
                "内置 skill `{}` 不应重复注册",
                skill.name
            );
        }
    }

    #[test]
    fn registry_contains_only_cli_workflow_guides() {
        let names = available()
            .iter()
            .map(|skill| skill.name)
            .collect::<Vec<_>>();
        assert_eq!(names, ["project-init", "doc-ingest", "gap-analysis"]);
    }
}
