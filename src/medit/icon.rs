use eframe::egui::epaint;
use eframe::egui::epaint::text::{FontFamily, LayoutJob, TextFormat};
use eframe::egui::{Pos2, Rect, Sense, Ui, Vec2};

use super::md::LinkInfo;

#[allow(non_camel_case_types)]
#[derive(Clone)]
pub enum IconName {
    icon_chevron_left,      //e900
    icon_chevron_down,      //e901
    icon_format_font_size,  //e902
    icon_wrap_text,         //e903
    icon_sort_numerically,  //e904
    icon_external_link(LinkInfo),     //e905
    icon_home, //e906
    icon_refresh, //e907
    icon_file_rename, //e908
    icon_delete, //e909
    icon_new, //e90A
    icon_unfixed, //e90B
    icon_fixed, //e90C
    icon_close, //e90d
}

impl IconName {
    pub fn to_char(&self) -> char {
        match self {
            IconName::icon_chevron_left  => '\u{e900}',
            IconName::icon_chevron_down => '\u{e901}',
            IconName::icon_format_font_size => '\u{e902}',
            IconName::icon_wrap_text => '\u{e903}', 
            IconName::icon_sort_numerically => '\u{e904}',
            IconName::icon_external_link(_) => '\u{e905}', 
            IconName::icon_home => '\u{e906}', 
            IconName::icon_refresh => '\u{e907}', //e907
            IconName::icon_file_rename => '\u{e908}', //e908
            IconName::icon_delete => '\u{e909}', //e909
            IconName::icon_new => '\u{e90A}', //e90A
            IconName::icon_unfixed => '\u{e90B}', //e90C
            IconName::icon_fixed => '\u{e90C}', //e90C
            IconName::icon_close => '\u{e90D}', //e90D
        }
    }
}

fn icon_job(ui: &Ui, icon_char: char, font_size: f32) -> LayoutJob {
    let mut job: LayoutJob = LayoutJob::default();
    let mut format = TextFormat::default();
    format.font_id.size = font_size;
    format.font_id.family = FontFamily::Name("icon".into());
    format.color = ui.visuals().weak_text_color();

    job.append(&icon_char.to_string(), 0.0, format);
    job
}

fn icon_job_selected(ui: &Ui, icon_char: char, font_size: f32) -> LayoutJob {
    let mut job: LayoutJob = LayoutJob::default();
    let mut format = TextFormat::default();
    format.font_id.size = font_size;
    format.font_id.family = FontFamily::Name("icon".into());
    format.color = ui.visuals().text_color();

    job.append(&icon_char.to_string(), 0.0, format);
    job
}

pub fn icon_size(ui: &mut Ui, icon_name: IconName, font_size: f32) -> Vec2 {
    let icon_char = icon_name.to_char();
    let job = icon_job(ui, icon_char, font_size);
    let galley = ui.fonts(|f| f.layout_job(job));
    galley.size() + Vec2 { x: 2.0, y: 2.0 }
}

//return if clicked
pub fn icon_button(ui: &mut Ui, id: String, pos: Pos2, icon_name: IconName, font_size: f32) -> bool {
    let icon_char = icon_name.to_char();
    let job = icon_job(ui, icon_char, font_size);
    let job_selected = icon_job_selected(ui, icon_char, font_size);
    let galley = ui.fonts(|f| f.layout_job(job));
    let galley_rect = Rect::from_min_size(pos, galley.size());
    let galley_rect = galley_rect.expand(2.0);
    let color = ui.visuals().weak_text_color();

    let responese = ui.interact(galley_rect, id.clone().into(), Sense::click());
    if responese.hovered() {
        //ui.painter().rect_stroke(galley_rect, 1.0, Stroke::new(1.0, color));
        let galley = ui.fonts(|f| f.layout_job(job_selected));
        ui.painter().add(epaint::TextShape::new(pos, galley, color));
    } else {
        ui.painter().add(epaint::TextShape::new(pos, galley, color));
    }
    if responese.clicked() {
        println!("clicked: {}", id);
        return true;
    }
    return false;
}
