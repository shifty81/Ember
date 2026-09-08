use super::render_helpers::{draw_editor_widget, draw_editor_widget_tone, WidgetTone};
use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GuiDocumentKind { Hud, Inventory, WorldCreation, Dialogue, GenericPanel }
impl GuiDocumentKind {
    fn label(self) -> &'static str { match self { Self::Hud=>"Gameplay HUD", Self::Inventory=>"Character Inventory", Self::WorldCreation=>"New World Creation", Self::Dialogue=>"Dialogue", Self::GenericPanel=>"Generic Panel" } }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GuiWidgetKind { Panel, NineSlice, Button, Tab, Slot, BarFrame, PortraitFrame, MinimapFrame, Label, ScrollArea }
impl GuiWidgetKind { fn label(self)->&'static str { match self { Self::Panel=>"Panel", Self::NineSlice=>"Nine-slice Frame", Self::Button=>"Button", Self::Tab=>"Tab", Self::Slot=>"Inventory Slot", Self::BarFrame=>"Empty Runtime Bar", Self::PortraitFrame=>"Portrait Frame", Self::MinimapFrame=>"Minimap Frame", Self::Label=>"Label", Self::ScrollArea=>"Scroll Area" } } }

pub(crate) struct GuiStudioState {
    document: GuiDocumentKind,
    widget: GuiWidgetKind,
    pub snap: bool,
    pub safe_area: bool,
    pub runtime_fill_preview: bool,
}
impl GuiStudioState {
    pub(crate) fn new() -> Self { Self { document: GuiDocumentKind::Hud, widget: GuiWidgetKind::Panel, snap: true, safe_area: true, runtime_fill_preview: false } }
    fn documents() -> [GuiDocumentKind;5] { [GuiDocumentKind::Hud,GuiDocumentKind::Inventory,GuiDocumentKind::WorldCreation,GuiDocumentKind::Dialogue,GuiDocumentKind::GenericPanel] }
    fn widgets() -> [GuiWidgetKind;10] { [GuiWidgetKind::Panel,GuiWidgetKind::NineSlice,GuiWidgetKind::Button,GuiWidgetKind::Tab,GuiWidgetKind::Slot,GuiWidgetKind::BarFrame,GuiWidgetKind::PortraitFrame,GuiWidgetKind::MinimapFrame,GuiWidgetKind::Label,GuiWidgetKind::ScrollArea] }
}

impl EditorApp {
    pub(crate) fn draw_gui_document_list(&self, rect: Rect) {
        let mut y=rect.y+4.0;
        draw_editor_text("Screens",rect.x,y+16.0,18.0,TEXT); y+=28.0;
        for document in GuiStudioState::documents() {
            let row=Rect::new(rect.x,y,rect.w,34.0);
            draw_editor_widget(row,document.label(),self.gui_studio.document==document);
            y+=40.0;
        }
        y+=8.0; draw_editor_text("Reusable Widgets",rect.x,y+16.0,18.0,TEXT); y+=28.0;
        for widget in GuiStudioState::widgets() {
            let row=Rect::new(rect.x,y,rect.w,30.0);
            draw_editor_widget_tone(row,widget.label(),self.gui_studio.widget==widget,WidgetTone::Quiet);
            y+=34.0;
            if y>rect.y+rect.h-28.0 { break; }
        }
    }

    pub(crate) fn draw_gui_studio(&self, rect: Rect) {
        let canvas=Rect::new(rect.x+18.0,rect.y+18.0,rect.w-36.0,rect.h-36.0);
        draw_rectangle(canvas.x,canvas.y,canvas.w,canvas.h,Color::new(0.035,0.04,0.05,1.0));
        if self.gui_studio.safe_area { draw_rectangle_lines(canvas.x+24.0,canvas.y+24.0,canvas.w-48.0,canvas.h-48.0,1.0,Color::new(0.25,0.55,0.42,0.7)); }
        let title=format!("{} — 1920×1080 reference",self.gui_studio.document.label());
        draw_editor_text(&title,canvas.x+18.0,canvas.y+28.0,18.0,TEXT);
        match self.gui_studio.document {
            GuiDocumentKind::Hud => self.draw_gui_hud_preview(canvas),
            GuiDocumentKind::Inventory => self.draw_gui_inventory_preview(canvas),
            GuiDocumentKind::WorldCreation => self.draw_gui_world_creation_preview(canvas),
            _ => self.draw_gui_generic_preview(canvas),
        }
    }

    fn rustic_panel(rect:Rect) { draw_rectangle(rect.x,rect.y,rect.w,rect.h,Color::new(0.16,0.09,0.045,0.98)); draw_rectangle_lines(rect.x,rect.y,rect.w,rect.h,3.0,Color::new(0.48,0.29,0.12,1.0)); }
    fn draw_gui_hud_preview(&self,c:Rect) {
        let portrait=Rect::new(c.x+38.0,c.y+58.0,88.0,96.0); Self::rustic_panel(portrait);
        for i in 0..3 { let r=Rect::new(c.x+134.0,c.y+62.0+i as f32*30.0,230.0,20.0); Self::rustic_panel(r); }
        let map=Rect::new(c.x+c.w-230.0,c.y+52.0,180.0,150.0); Self::rustic_panel(map);
        let hot=Rect::new(c.x+c.w*0.5-270.0,c.y+c.h-102.0,540.0,62.0); Self::rustic_panel(hot);
        for i in 0..8 { let s=Rect::new(hot.x+18.0+i as f32*63.0,hot.y+10.0,52.0,42.0); Self::rustic_panel(s); }
        draw_editor_text("Runtime portrait",portrait.x+8.0,portrait.y+52.0,12.0,MUTED); draw_editor_text("Runtime minimap + clock",map.x+12.0,map.y+76.0,14.0,MUTED);
    }
    fn draw_gui_inventory_preview(&self,c:Rect) {
        let frame=Rect::new(c.x+58.0,c.y+58.0,c.w-116.0,c.h-112.0); Self::rustic_panel(frame);
        let stats=Rect::new(frame.x+18.0,frame.y+42.0,220.0,frame.h-72.0); Self::rustic_panel(stats);
        let paper=Rect::new(stats.x+238.0,stats.y,270.0,stats.h); Self::rustic_panel(paper);
        let inv=Rect::new(paper.x+288.0,paper.y,frame.w-562.0,paper.h); Self::rustic_panel(inv);
        for y in 0..5 { for x in 0..7 { let r=Rect::new(inv.x+16.0+x as f32*48.0,inv.y+58.0+y as f32*48.0,40.0,40.0); Self::rustic_panel(r); } }
        draw_editor_text("Character / equipment",paper.x+42.0,paper.y+30.0,18.0,TEXT); draw_editor_text("Inventory grid",inv.x+16.0,inv.y+30.0,18.0,TEXT);
    }
    fn draw_gui_world_creation_preview(&self,c:Rect) {
        let frame=Rect::new(c.x+58.0,c.y+54.0,c.w-116.0,c.h-108.0); Self::rustic_panel(frame);
        let tabs=Rect::new(frame.x+18.0,frame.y+48.0,170.0,frame.h-78.0); Self::rustic_panel(tabs);
        let options=Rect::new(tabs.x+188.0,tabs.y,frame.w-520.0,tabs.h); Self::rustic_panel(options);
        let preview=Rect::new(options.x+options.w+18.0,tabs.y,290.0,190.0); Self::rustic_panel(preview);
        draw_editor_text("Basic\nWorld Gen\nGameplay\nAdvanced",tabs.x+24.0,tabs.y+42.0,18.0,TEXT);
        draw_editor_text("World Name  |  Seed\nMode: Story / Sandbox / Custom\nSize, islands, season, weather\nresources, NPC density, danger",options.x+22.0,options.y+42.0,18.0,TEXT);
        draw_editor_text("Live world preview",preview.x+54.0,preview.y+100.0,16.0,MUTED);
    }
    fn draw_gui_generic_preview(&self,c:Rect) { let r=Rect::new(c.x+c.w*.2,c.y+c.h*.2,c.w*.6,c.h*.6); Self::rustic_panel(r); draw_editor_text("Reusable panel composition",r.x+40.0,r.y+50.0,22.0,TEXT); }

    pub(crate) fn draw_gui_inspector(&self,rect:Rect) {
        let mut y=rect.y+10.0; draw_editor_text("GUI Studio",rect.x,y+18.0,22.0,TEXT); y+=38.0;
        for (label,on) in [("8 px snap",self.gui_studio.snap),("Safe area",self.gui_studio.safe_area),("Runtime fill preview",self.gui_studio.runtime_fill_preview)] { draw_editor_text(&format!("{}: {}",label,if on{"On"}else{"Off"}),rect.x,y+16.0,16.0,if on{GOOD}else{MUTED}); y+=28.0; }
        y+=8.0; draw_editor_text("Production rules",rect.x,y+16.0,18.0,TEXT); y+=28.0;
        for line in ["Frames use 9-slice metadata","Bars remain empty artwork","Engine owns fill/values","Portrait and minimap are live","Text is never baked into art","One shared component catalog"] { draw_editor_text(line,rect.x,y+14.0,14.0,MUTED); y+=22.0; }
        y+=12.0; draw_editor_text("Source kit",rect.x,y+16.0,18.0,TEXT); y+=26.0;
        draw_editor_text("content/ui/gui/havenwild_rustic_gui_kit_v1.png",rect.x,y+14.0,12.0,MUTED);
    }

    pub(crate) fn handle_gui_studio_click(&mut self,mx:f32,my:f32)->bool {
        if self.handle_editor_menu_click(mx,my) { return true; }
        let layout=self.shell_layout();
        let p=vec2(mx,my); let mut y=layout.list_content.y+32.0;
        for doc in GuiStudioState::documents() { let r=Rect::new(layout.list_content.x,y,layout.list_content.w,34.0); if r.contains(p) { self.gui_studio.document=doc; self.status_message=format!("GUI Studio: {}",doc.label()); return true; } y+=40.0; }
        y+=36.0; for widget in GuiStudioState::widgets() { let r=Rect::new(layout.list_content.x,y,layout.list_content.w,30.0); if r.contains(p) { self.gui_studio.widget=widget; self.status_message=format!("Selected reusable GUI widget: {}",widget.label()); return true; } y+=34.0; }
        false
    }
}
